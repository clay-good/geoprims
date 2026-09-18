//! Host-supplied data assets (compute-core "Asset provided by host"). The core
//! never does I/O: a tool asks for `id@version/key`; when the host has not
//! supplied it, the call fails with `ASSET_UNAVAILABLE` naming exactly what is
//! missing, and the host fetches it, checks its SHA-256 against the asset
//! registry, supplies it with `gp_asset_put`, and retries.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

thread_local! {
    static STORE: RefCell<BTreeMap<String, Rc<[u8]>>> = const { RefCell::new(BTreeMap::new()) };
}

/// The store key for one asset file or tile.
pub fn key(id: &str, version: &str, file: &str) -> String {
    format!("{id}@{version}/{file}")
}

/// Supplies an asset's bytes (already integrity-checked by the host).
pub fn put(key: &str, bytes: &[u8]) {
    STORE.with(|s| s.borrow_mut().insert(key.to_owned(), Rc::from(bytes)));
}

pub fn get(key: &str) -> Option<Rc<[u8]>> {
    STORE.with(|s| s.borrow().get(key).cloned())
}

/// Drops every supplied asset (the host's cache eviction).
pub fn clear() {
    STORE.with(|s| s.borrow_mut().clear());
}
