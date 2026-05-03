//! wasm32 + `web_audio`: call `window.LearnOpenglFmod` from [`web/game_fmod_bridge.js`](../../../web/game_fmod_bridge.js).

use js_sys::{Array, Reflect};
use wasm_bindgen::{JsCast, JsValue};

fn bridge_object() -> Option<JsValue> {
    let window = web_sys::window()?;
    let v = Reflect::get(&window, &JsValue::from_str("LearnOpenglFmod")).ok()?;
    if v.is_undefined() || v.is_null() {
        return None;
    }
    Some(v)
}

pub fn bridge_is_ready() -> bool {
    let Some(b) = bridge_object() else {
        return false;
    };
    let Ok(m) = Reflect::get(&b, &JsValue::from_str("isReady")) else {
        return false;
    };
    if !m.is_function() {
        return false;
    }
    let f: &js_sys::Function = m.unchecked_ref();
    f.call0(&b)
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn call_bridge(method: &str, args: &[JsValue]) {
    let Some(b) = bridge_object() else {
        return;
    };
    let Ok(m) = Reflect::get(&b, &JsValue::from_str(method)) else {
        return;
    };
    if !m.is_function() {
        return;
    }
    let f: &js_sys::Function = m.unchecked_ref();
    let arr = Array::new();
    for a in args {
        arr.push(a);
    }
    let _ = f.apply(&b, &arr);
}
