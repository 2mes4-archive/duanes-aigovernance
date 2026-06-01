use axum::http::HeaderMap;

pub fn extract_identity(headers: &HeaderMap) -> Option<duanes_common::models::CallerIdentity> {
    let user_id = headers.get("X-Duanes-User-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())?;

    Some(duanes_common::models::CallerIdentity {
        user_id,
        app_id: headers.get("X-Duanes-App-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        team_id: headers.get("X-Duanes-Team-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        org_id: headers.get("X-Duanes-Org-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    })
}
