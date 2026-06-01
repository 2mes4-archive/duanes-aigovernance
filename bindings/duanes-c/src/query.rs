use crate::DuanesEngine;
use crate::internal;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn duanes_query_recent(
    engine: *mut DuanesEngine,
    limit: u32,
) -> *mut c_char {
    if engine.is_null() {
        return ptr::null_mut();
    }

    let engine = unsafe { &*engine };
    let rt = internal::runtime();

    let records = match rt.block_on(engine.storage.query_recent(limit as u64)) {
        Ok(r) => r,
        Err(_) => return ptr::null_mut(),
    };

    let json = match serde_json::to_string(&records) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    CString::into_raw(CString::new(json).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn duanes_query_daily_summary(
    engine: *mut DuanesEngine,
    user_id: *const c_char,
) -> *mut c_char {
    if engine.is_null() {
        return ptr::null_mut();
    }

    let engine = unsafe { &*engine };
    let rt = internal::runtime();

    let uid = if user_id.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(user_id) }.to_str().ok()
    };

    let summary = match rt.block_on(engine.storage.get_daily_summary(uid)) {
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
