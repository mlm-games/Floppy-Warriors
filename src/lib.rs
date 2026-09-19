pub mod sim;

mod app;
mod render;
mod ui;

pub use app::App;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    let result = app::run_desktop();
    if let Err(e) = result {
        eprintln!("floppy-warriors failed to start: {e:?}");
    }
}
