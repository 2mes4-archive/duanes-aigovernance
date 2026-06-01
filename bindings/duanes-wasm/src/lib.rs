use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct DuanesWasm {
    config: duanes_common::DuanesConfig,
}

#[wasm_bindgen]
impl DuanesWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<DuanesWasm, JsValue> {
        let config: duanes_common::DuanesConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Config error: {}", e)))?;
        Ok(Self { config })
    }

    #[wasm_bindgen]
    pub fn inference(&self, request_json: &str, _user_id: Option<String>) -> Result<String, JsValue> {
        let _request: duanes_common::models::InferenceRequest = serde_json::from_str(request_json)
            .map_err(|e| JsValue::from_str(&format!("Request parse error: {}", e)))?;

        let _ = &self.config;

        let response = serde_json::json!({
            "error": "WASM inference is not yet implemented. Use the HTTP proxy instead."
        });
        Ok(serde_json::to_string(&response).unwrap_or_default())
    }

    #[wasm_bindgen]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
