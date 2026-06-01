use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

pub async fn request_logger(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let elapsed = start.elapsed();
    tracing::info!(
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        elapsed_ms = elapsed.as_millis(),
        "Request processed"
    );

    Ok(response)
}
