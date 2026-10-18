#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
fn main() {
    floppy_warriors::run();
}

#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn main() {}
