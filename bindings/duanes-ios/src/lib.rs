uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn duanes_init_ios(config_path: String) -> Result<String, String> {
    let _config = duanes_common::DuanesConfig::from_file(&config_path)
        .map_err(|e| format!("Config error: {}", e))?;
    Ok("initialized".to_string())
}

#[uniffi::export]
pub fn duanes_inference_ios(request_json: String, user_id: Option<String>) -> Result<String, String> {
    let _request: duanes_common::models::InferenceRequest = serde_json::from_str(&request_json)
        .map_err(|e| format!("Request parse error: {}", e))?;

    let response = serde_json::json!({
        "id": "stub",
        "object": "chat.completion",
        "model": "stub",
        "choices": [],
        "usage": { "prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0 }
    });

    Ok(serde_json::to_string(&response).unwrap_or_default())
}
