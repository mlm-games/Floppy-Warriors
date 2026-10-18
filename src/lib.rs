pub mod sim;

mod app;
mod render;
mod ui;
mod view2d;

pub use app::App;

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn run() {
    if let Err(error) = app::run_desktop() {
        eprintln!("floppy-warriors failed to start: {error:?}");
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run() {
    if let Err(error) = app::run_web() {
        eprintln!("floppy-warriors failed to start: {error:?}");
    }
}
