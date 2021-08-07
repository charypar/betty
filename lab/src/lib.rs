use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PriceRecord {
    #[serde(rename = "Open")]
    open: f32,
    #[serde(rename = "High")]
    high: f32,
    #[serde(rename = "Low")]
    low: f32,
    #[serde(rename = "Close")]
    close: f32,
    #[serde(rename = "Volume")]
    volume: f32,
}

#[wasm_bindgen]
pub fn reflect(record: JsValue) -> JsValue {
    if let Ok(value) = record.into_serde::<Vec<PriceRecord>>() {
        JsValue::from_serde(&value).unwrap()
    } else {
        let value: Vec<PriceRecord> = vec![];
        JsValue::from_serde(&value).unwrap()
    }
}

#[wasm_bindgen]
pub fn to_string(record: JsValue) -> String {
    match record.into_serde::<Vec<PriceRecord>>() {
        Ok(record) => format!("{:#?}!", record),
        Err(err) => format!("{}", err),
    }
}
