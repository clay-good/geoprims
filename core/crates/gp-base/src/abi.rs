//! The raw WebAssembly ABI every module exports (compute-core "Stable host call
//! interface"). Strings cross the boundary as UTF-8 in linear memory:
//!
//! - `gp_invoke(id, idLen, input, inputLen)`, `gp_invoke_batch(…)`, `gp_manifest()`,
//!   and `gp_version()` return JSON (or the version string).
//! - `gp_alloc(len) -> ptr` / `gp_free(ptr, len)`: host-owned input buffers.
//! - Calls that return a string return a pointer to a core-owned buffer and set
//!   its length, read with `gp_out_len()`. The buffer lives until the next call.
//!
//! There is no generated JS glue, so modules have an empty import section and a
//! single ~50-line loader serves the browser and Node.

use std::cell::RefCell;

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

pub fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr` must come from [`alloc`] with the same `len`, and not be freed twice.
pub unsafe fn free(ptr: *mut u8, len: usize) {
    // SAFETY: the caller guarantees ptr/len came from `alloc`.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
}

/// Stores `s` as the current output and returns its pointer.
pub fn set_out(s: &str) -> *const u8 {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        o.clear();
        o.extend_from_slice(s.as_bytes());
        o.as_ptr()
    })
}

pub fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

/// # Safety
/// `ptr` must point to `len` readable bytes.
pub unsafe fn read_str<'a>(ptr: *const u8, len: usize) -> Result<&'a str, std::str::Utf8Error> {
    // SAFETY: the caller guarantees the range is valid for the call's duration.
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) })
}

pub fn bad_utf8() -> String {
    crate::envelope::failure(&crate::error::ToolError::new(
        crate::error::ErrorCode::InvalidInput,
        "The tool id and input must be UTF-8.",
    ))
}

/// Declares a module's Wasm exports. `$name` is the module id (`base`, `geodesy`, …).
#[macro_export]
macro_rules! export_module {
    ($name:literal, $registry:expr) => {
        #[cfg(target_arch = "wasm32")]
        mod __gp_exports {
            #[allow(unused_imports)]
            use super::*;

            #[unsafe(no_mangle)]
            pub extern "C" fn gp_alloc(len: usize) -> *mut u8 {
                $crate::abi::alloc(len)
            }

            /// # Safety
            /// See [`gp_base::abi::free`].
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn gp_free(ptr: *mut u8, len: usize) {
                unsafe { $crate::abi::free(ptr, len) }
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn gp_out_len() -> usize {
                $crate::abi::out_len()
            }

            /// Supplies an asset: `key` is `id@version/file`, `data` its bytes.
            ///
            /// # Safety
            /// Both ranges must be readable buffers from `gp_alloc`.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn gp_asset_put(
                key: *const u8,
                key_len: usize,
                data: *const u8,
                data_len: usize,
            ) -> i32 {
                match unsafe { $crate::abi::read_str(key, key_len) } {
                    Ok(k) => {
                        // SAFETY: the host passes a buffer it allocated with gp_alloc.
                        let bytes = unsafe { core::slice::from_raw_parts(data, data_len) };
                        $crate::assets::put(k, bytes);
                        0
                    }
                    Err(_) => 1,
                }
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn gp_asset_clear() {
                $crate::assets::clear()
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn gp_version() -> *const u8 {
                $crate::abi::set_out(concat!($name, "@", env!("CARGO_PKG_VERSION")))
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn gp_manifest() -> *const u8 {
                $crate::abi::set_out(&$registry.manifest())
            }

            /// # Safety
            /// Both ranges must be readable UTF-8 buffers from `gp_alloc`.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn gp_invoke(
                id: *const u8,
                id_len: usize,
                input: *const u8,
                input_len: usize,
            ) -> *const u8 {
                let out = match unsafe {
                    (
                        $crate::abi::read_str(id, id_len),
                        $crate::abi::read_str(input, input_len),
                    )
                } {
                    (Ok(id), Ok(input)) => $registry.invoke(id, input),
                    _ => $crate::abi::bad_utf8(),
                };
                $crate::abi::set_out(&out)
            }

            /// # Safety
            /// Both ranges must be readable UTF-8 buffers from `gp_alloc`.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn gp_invoke_batch(
                id: *const u8,
                id_len: usize,
                input: *const u8,
                input_len: usize,
            ) -> *const u8 {
                let out = match unsafe {
                    (
                        $crate::abi::read_str(id, id_len),
                        $crate::abi::read_str(input, input_len),
                    )
                } {
                    (Ok(id), Ok(input)) => $registry.invoke_batch(id, input),
                    _ => $crate::abi::bad_utf8(),
                };
                $crate::abi::set_out(&out)
            }
        }
    };
}
