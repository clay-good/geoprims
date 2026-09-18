#[link(wasm_import_module = "Math")]
unsafe extern "C" {
    fn sin(x: f64) -> f64;
}

#[unsafe(no_mangle)]
pub extern "C" fn uses_host_math(x: f64) -> f64 {
    // SAFETY: a host import; this fixture exists only to fail the lint.
    unsafe { sin(x) }
}
