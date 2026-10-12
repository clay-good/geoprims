//! Ephemeral bytes from files a user explicitly selected.
//!
//! Files are separate from public, integrity-checked [`crate::assets`]. The
//! host assigns an opaque handle, supplies the bytes, and puts only that handle
//! in tool JSON. Nothing here performs I/O or persists user data.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Whole-file bridge limit. Larger COGs use the range-fed path instead.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_HANDLE_BYTES: usize = 128;

thread_local! {
    static STORE: RefCell<BTreeMap<String, Rc<[u8]>>> = const { RefCell::new(BTreeMap::new()) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PutError {
    InvalidHandle,
    LimitExceeded,
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
            .insert(handle.to_owned(), Rc::from(bytes));
    });
    Ok(())
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
    STORE.with(|store| store.borrow().get(handle).cloned())
}

pub fn remove(handle: &str) {
    STORE.with(|store| {
        store.borrow_mut().remove(handle);
    });
}

pub fn clear() {
    STORE.with(|store| store.borrow_mut().clear());
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
}
