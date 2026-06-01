use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

pub mod types;
pub mod engine;
pub mod query;

pub use types::*;
pub use engine::*;
pub use query::*;

mod internal {
    use std::sync::OnceLock;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    pub static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    pub fn runtime() -> &'static Runtime {
        RUNTIME.get_or_init(|| {
            Runtime::new().expect("Failed to create tokio runtime")
        })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn duanes_init(config_path: *const c_char) -> *mut DuanesEngine {
    if config_path.is_null() {
        return ptr::null_mut();
    }

    let config_path_str = match unsafe { CStr::from_ptr(config_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let config = match duanes_common::DuanesConfig::from_file(config_path_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load config");
            return ptr::null_mut();
        }
    };

    let rt = internal::runtime();

    let storage = match rt.block_on(async {
        duanes_storage::SqliteStorage::new(&config.storage)
    }) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::error!(error = %e, "Failed to initialize storage");
            return ptr::null_mut();
        }
    };

    let provider_registry = Arc::new(duanes_core::ProviderRegistry::new());
    let governance = Arc::new(duanes_core::GovernanceEngine::new(config.governance.clone()));

    let router = Arc::new(duanes_core::InferenceRouter::new(
        provider_registry,
        governance,
        config.engine.default_provider.clone(),
    ));

    let engine = DuanesEngine {
        router,
        storage,
        config: Arc::new(config),
    };

    Box::into_raw(Box::new(engine))
}

#[unsafe(no_mangle)]
pub extern "C" fn duanes_destroy(engine: *mut DuanesEngine) {
    if !engine.is_null() {
        unsafe { drop(Box::from_raw(engine)) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn duanes_inference(
    engine: *mut DuanesEngine,
    request_json: *const c_char,
    user_id: *const c_char,
) -> *mut c_char {
    if engine.is_null() || request_json.is_null() {
        return ptr::null_mut();
    }

    let engine = unsafe { &*engine };
    let request_str = match unsafe { CStr::from_ptr(request_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let request: duanes_common::models::InferenceRequest = match serde_json::from_str(request_str) {
        Ok(r) => r,
        Err(_) => return ptr::null_mut(),
    };

    let identity = if user_id.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(user_id) }.to_str() {
            Ok(uid) => Some(duanes_common::models::CallerIdentity {
                user_id: uid.to_string(),
                app_id: None,
                team_id: None,
                org_id: None,
            }),
            Err(_) => None,
        }
    };

    let rt = internal::runtime();
    let result = match rt.block_on(engine.router.route(request, identity)) {
        Ok(r) => r,
        Err(_) => return ptr::null_mut(),
    };

    let _ = rt.block_on(engine.storage.store(&result.record));

    let response_json = match serde_json::to_string(&result.response) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    CString::into_raw(CString::new(response_json).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn duanes_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn duanes_get_stats(engine: *mut DuanesEngine) -> *mut c_char {
    if engine.is_null() {
        return ptr::null_mut();
    }

    let engine = unsafe { &*engine };
    let rt = internal::runtime();

    let summary = match rt.block_on(engine.storage.get_daily_summary(None)) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let json = serde_json::json!({
        "total_requests": summary.total_requests,
        "total_tokens": summary.total_tokens,
        "total_cost": summary.total_cost,
        "avg_latency_ms": summary.avg_latency_ms,
    });

    let json_str = serde_json::to_string(&json).unwrap_or_default();
    CString::into_raw(CString::new(json_str).unwrap_or_default())
}
