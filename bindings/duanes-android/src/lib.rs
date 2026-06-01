use jni::objects::{JClass, JString, JObject};
use jni::sys::{jstring, jint};
use jni::JNIEnv;

#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_duanes_DuanesNative_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    config_path: JString,
) -> jint {
    let config_path: String = match env.get_string(&config_path) {
        Ok(s) => s.into(),
        Err(_) => return -1,
    };

    0
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_duanes_DuanesNative_nativeInference(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
    user_id: JString,
) -> jstring {
    let request_str: String = match env.get_string(&request_json) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let response = serde_json::json!({
        "id": "stub",
        "object": "chat.completion",
        "model": "stub",
        "choices": [],
        "usage": { "prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0 }
    });

    let response_str = serde_json::to_string(&response).unwrap_or_default();
    env.new_string(response_str)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
