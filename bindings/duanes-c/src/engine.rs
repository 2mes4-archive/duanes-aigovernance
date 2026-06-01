use crate::DuanesEngine;
use crate::internal;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn duanes_register_provider(
    engine: *mut DuanesEngine,
    name: *const c_char,
    base_url: *const c_char,
    api_key: *const c_char,
    models_json: *const c_char,
) -> i32 {
    if engine.is_null() || name.is_null() || base_url.is_null() || api_key.is_null() {
        return -1;
    }

    let engine = unsafe { &*engine };
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    let base_url_str = unsafe { CStr::from_ptr(base_url) }.to_str().unwrap_or("");
    let api_key_str = unsafe { CStr::from_ptr(api_key) }.to_str().unwrap_or("");

    let models: Vec<duanes_common::models::ModelPricing> = if models_json.is_null() {
        Vec::new()
    } else {
        let models_str = unsafe { CStr::from_ptr(models_json) }.to_str().unwrap_or("[]");
        serde_json::from_str(models_str).unwrap_or_default()
    };

    let provider_config = duanes_common::models::ProviderConfig {
        name: name_str.to_string(),
        base_url: base_url_str.to_string(),
        api_key: api_key_str.to_string(),
        models,
        rate_limit: None,
        timeout_secs: None,
    };

    let rt = internal::runtime();
    match rt.block_on(engine.storage.query_recent(50)) {
        Ok(_) => {}
        Err(_) => {}
    }

    0
}
