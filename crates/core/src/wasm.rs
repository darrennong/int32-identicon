use wasm_bindgen::prelude::*;

fn js_error(e: crate::Error) -> JsValue {
    JsValue::from_str(&e.to_string())
}
fn color(foreground: i32) -> Result<Option<u8>, JsValue> {
    if foreground == -1 {
        Ok(None)
    } else if (0..16).contains(&foreground) {
        Ok(Some(foreground as u8))
    } else {
        Err(JsValue::from_str("invalid foreground"))
    }
}
fn result_json(d: crate::Decoded) -> String {
    format!(
        "{{\"value\":{},\"foreground\":{},\"repairedBytes\":{},\"version\":1}}",
        d.value, d.foreground, d.repaired_bytes
    )
}

#[wasm_bindgen]
pub fn encode_svg(value: i32, foreground: i32) -> Result<String, JsValue> {
    crate::svg(value, color(foreground)?).map_err(js_error)
}

#[wasm_bindgen]
pub fn decode_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<String, JsValue> {
    crate::scan::decode_image(width as usize, height as usize, pixels)
        .map(result_json)
        .map_err(js_error)
}

#[wasm_bindgen]
pub fn decode_quad_rgba(
    width: u32,
    height: u32,
    pixels: &[u8],
    corners: &[f64],
) -> Result<String, JsValue> {
    if corners.len() != 8 {
        return Err(JsValue::from_str("expected eight corner coordinates"));
    }
    let quad = std::array::from_fn(|i| [corners[i * 2], corners[i * 2 + 1]]);
    crate::scan::decode_quad(width as usize, height as usize, pixels, quad)
        .map(result_json)
        .map_err(js_error)
}
