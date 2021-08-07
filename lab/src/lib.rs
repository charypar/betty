use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(subject: String) -> String {
    format!("Hello, {}!", subject)
}
