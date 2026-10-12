//! Ephemeral bytes from files a user explicitly selected.
//!
//! Files are separate from public, integrity-checked [`crate::assets`]. The
//! host assigns an opaque handle, supplies the bytes, and puts only that handle
//! in tool JSON. Nothing here performs I/O or persists user data.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::rc::Rc;

/// Whole-file bridge limit. Larger COGs use the range-fed path instead.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SPARSE_BYTES: u64 = 16 * 1024 * 1024 * 1024;
pub const MAX_RANGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_HANDLE_BYTES: usize = 128;
const RANGE_PAGE_BYTES: u64 = 64 * 1024;

thread_local! {
    static STORE: RefCell<BTreeMap<String, Entry>> = const { RefCell::new(BTreeMap::new()) };
    static MISSING: RefCell<Option<MissingRange>> = const { RefCell::new(None) };
}

#[derive(Clone)]
enum Entry {
    Whole(Rc<[u8]>),
    Sparse(Rc<RefCell<SparseFile>>),
}

struct SparseFile {
    len: u64,
    cached: usize,
    ranges: BTreeMap<u64, Rc<[u8]>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingRange {
    pub handle: String,
    pub offset: u64,
    pub length: usize,
}

#[derive(Clone)]
pub struct FileReader {
    handle: String,
    entry: Entry,
    position: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PutError {
    InvalidHandle,
    LimitExceeded,
    NotSparse,
    OutOfBounds,
}

fn valid_handle(handle: &str) -> bool {
    !handle.is_empty()
        && handle.len() <= MAX_HANDLE_BYTES
        && handle
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Copies one bounded file into module memory under an opaque host handle.
pub fn put(handle: &str, bytes: &[u8]) -> Result<(), PutError> {
    check(handle, bytes.len())?;
    STORE.with(|store| {
        store
            .borrow_mut()
            .insert(handle.to_owned(), Entry::Whole(Rc::from(bytes)));
    });
    Ok(())
}

/// Declares a large private file without allocating its contents.
pub fn open_sparse(handle: &str, len: u64) -> Result<(), PutError> {
    if !valid_handle(handle) {
        return Err(PutError::InvalidHandle);
    }
    if len == 0 || len > MAX_SPARSE_BYTES {
        return Err(PutError::LimitExceeded);
    }
    STORE.with(|store| {
        store.borrow_mut().insert(
            handle.to_owned(),
            Entry::Sparse(Rc::new(RefCell::new(SparseFile {
                len,
                cached: 0,
                ranges: BTreeMap::new(),
            }))),
        );
    });
    Ok(())
}

/// Supplies one range from a previously declared sparse file.
pub fn put_range(handle: &str, offset: u64, bytes: &[u8]) -> Result<(), PutError> {
    if bytes.is_empty() || bytes.len() > MAX_RANGE_BYTES {
        return Err(PutError::LimitExceeded);
    }
    STORE.with(|store| {
        let entry = store
            .borrow()
            .get(handle)
            .cloned()
            .ok_or(PutError::NotSparse)?;
        let Entry::Sparse(file) = entry else {
            return Err(PutError::NotSparse);
        };
        let mut file = file.borrow_mut();
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or(PutError::OutOfBounds)?;
        if end > file.len {
            return Err(PutError::OutOfBounds);
        }
        let replaced = file.ranges.get(&offset).map_or(0, |old| old.len());
        let cached = file
            .cached
            .checked_sub(replaced)
            .and_then(|size| size.checked_add(bytes.len()))
            .ok_or(PutError::LimitExceeded)?;
        if cached > MAX_BYTES {
            return Err(PutError::LimitExceeded);
        }
        file.cached = cached;
        file.ranges.insert(offset, Rc::from(bytes));
        Ok(())
    })
}

fn check(handle: &str, len: usize) -> Result<(), PutError> {
    if !valid_handle(handle) {
        return Err(PutError::InvalidHandle);
    }
    if len > MAX_BYTES {
        return Err(PutError::LimitExceeded);
    }
    Ok(())
}

pub fn get(handle: &str) -> Option<Rc<[u8]>> {
    STORE.with(|store| match store.borrow().get(handle) {
        Some(Entry::Whole(bytes)) => Some(bytes.clone()),
        _ => None,
    })
}

/// Opens either whole-file bytes or a sparse, range-backed file.
pub fn reader(handle: &str) -> Option<FileReader> {
    MISSING.with(|missing| *missing.borrow_mut() = None);
    STORE.with(|store| {
        store.borrow().get(handle).cloned().map(|entry| FileReader {
            handle: handle.to_owned(),
            entry,
            position: 0,
        })
    })
}

pub fn missing_range() -> Option<MissingRange> {
    MISSING.with(|missing| missing.borrow_mut().take())
}

pub fn clear_ranges(handle: &str) {
    STORE.with(|store| {
        if let Some(Entry::Sparse(file)) = store.borrow().get(handle) {
            let mut file = file.borrow_mut();
            file.cached = 0;
            file.ranges.clear();
        }
    });
}

pub fn remove(handle: &str) {
    STORE.with(|store| {
        store.borrow_mut().remove(handle);
    });
}

pub fn clear() {
    STORE.with(|store| store.borrow_mut().clear());
    MISSING.with(|missing| *missing.borrow_mut() = None);
}

impl FileReader {
    fn len(&self) -> u64 {
        match &self.entry {
            Entry::Whole(bytes) => bytes.len() as u64,
            Entry::Sparse(file) => file.borrow().len,
        }
    }

    fn record_missing(&self, requested: usize) {
        let len = self.len();
        let offset = self.position / RANGE_PAGE_BYTES * RANGE_PAGE_BYTES;
        let needed_end = self
            .position
            .saturating_add(requested as u64)
            .max(offset.saturating_add(RANGE_PAGE_BYTES))
            .min(len);
        let length = needed_end
            .saturating_sub(offset)
            .min(MAX_RANGE_BYTES as u64) as usize;
        MISSING.with(|missing| {
            *missing.borrow_mut() = Some(MissingRange {
                handle: self.handle.clone(),
                offset,
                length,
            });
        });
    }
}

impl Read for FileReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.position >= self.len() {
            return Ok(0);
        }
        let available = (self.len() - self.position).min(buffer.len() as u64) as usize;
        match &self.entry {
            Entry::Whole(bytes) => {
                let start = self.position as usize;
                buffer[..available].copy_from_slice(&bytes[start..start + available]);
                self.position += available as u64;
                Ok(available)
            }
            Entry::Sparse(file) => {
                let file = file.borrow();
                let found = file
                    .ranges
                    .range(..=self.position)
                    .rev()
                    .find(|(start, bytes)| **start + bytes.len() as u64 > self.position);
                let Some((start, bytes)) = found else {
                    drop(file);
                    self.record_missing(available);
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "private file range is not loaded",
                    ));
                };
                let range_offset = (self.position - *start) as usize;
                let copied = available.min(bytes.len() - range_offset);
                buffer[..copied].copy_from_slice(&bytes[range_offset..range_offset + copied]);
                self.position += copied as u64;
                Ok(copied)
            }
        }
    }
}

impl Seek for FileReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let next = match position {
            SeekFrom::Start(position) => i128::from(position),
            SeekFrom::End(delta) => i128::from(self.len()) + i128::from(delta),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
        };
        if !(0..=i128::from(self.len())).contains(&next) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "private file seek is out of bounds",
            ));
        }
        self.position = next as u64;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_bounded_private_handles_and_can_be_released() {
        clear();
        assert_eq!(put("", b"x"), Err(PutError::InvalidHandle));
        assert_eq!(put("a/b", b"x"), Err(PutError::InvalidHandle));
        put("raster-1", b"first").unwrap();
        assert_eq!(&*get("raster-1").unwrap(), b"first");
        put("raster-1", b"replacement").unwrap();
        assert_eq!(&*get("raster-1").unwrap(), b"replacement");
        remove("raster-1");
        assert!(get("raster-1").is_none());
    }

    #[test]
    fn refuses_a_whole_file_over_the_bridge_limit() {
        assert_eq!(
            check("too-large", MAX_BYTES + 1),
            Err(PutError::LimitExceeded)
        );
    }

    #[test]
    fn sparse_files_request_and_read_only_supplied_ranges() {
        clear();
        open_sparse("large-cog", 1_000_000_000).unwrap();
        let mut reader = reader("large-cog").unwrap();
        reader.seek(SeekFrom::Start(70_000)).unwrap();
        let mut buffer = [0_u8; 4];
        assert_eq!(
            reader.read(&mut buffer).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            missing_range().unwrap(),
            MissingRange {
                handle: "large-cog".into(),
                offset: 65_536,
                length: 65_536,
            }
        );
        assert!(
            missing_range().is_none(),
            "a missing-range request was not consumed"
        );
        put_range("large-cog", 65_536, &vec![7_u8; 65_536]).unwrap();
        assert_eq!(reader.read(&mut buffer).unwrap(), 4);
        assert_eq!(buffer, [7; 4]);
        clear_ranges("large-cog");
        assert_eq!(
            reader.read(&mut buffer).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }

    #[test]
    fn sparse_ranges_are_bounded_without_allocating_the_declared_file() {
        clear();
        assert_eq!(open_sparse("empty", 0), Err(PutError::LimitExceeded));
        open_sparse("large-cog", MAX_SPARSE_BYTES).unwrap();
        assert_eq!(
            put_range("large-cog", MAX_SPARSE_BYTES - 1, &[1, 2]),
            Err(PutError::OutOfBounds)
        );
        assert_eq!(put_range("large-cog", 0, &[]), Err(PutError::LimitExceeded));
        put("whole", b"abc").unwrap();
        assert_eq!(put_range("whole", 0, b"a"), Err(PutError::NotSparse));
    }

    #[test]
    fn whole_files_also_use_the_seekable_reader() {
        clear();
        put("whole", b"abcdef").unwrap();
        let mut reader = reader("whole").unwrap();
        reader.seek(SeekFrom::End(-3)).unwrap();
        let mut tail = Vec::new();
        reader.read_to_end(&mut tail).unwrap();
        assert_eq!(tail, b"def");
        assert!(missing_range().is_none());
    }
}
