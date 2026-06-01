use duanes_common::errors::DuanesResult;
use duanes_common::models::InferenceRequest;
use serde_json::Value;
use std::time::Instant;

pub struct OpenAICompatLayer;

impl OpenAICompatLayer {
    pub fn parse_request(body: &[u8]) -> DuanesResult<InferenceRequest> {
        let mut value: Value = serde_json::from_slice(body)
            .map_err(|e| duanes_common::DuanesError::Serialization(e.to_string()))?;

        if let Some(obj) = value.as_object_mut() {
            obj.remove("stream");
        }

        let request: InferenceRequest = serde_json::from_value(value)
            .map_err(|e| duanes_common::DuanesError::Serialization(e.to_string()))?;

        Ok(request)
    }

    pub fn serialize_response(response: &duanes_common::models::InferenceResponse) -> DuanesResult<Vec<u8>> {
        serde_json::to_vec(response)
            .map_err(|e| duanes_common::DuanesError::Serialization(e.to_string()))
    }

    pub fn parse_stream_chunk(chunk: &[u8]) -> DuanesResult<Option<duanes_common::models::StreamChunk>> {
        let text = std::str::from_utf8(chunk)
            .map_err(|e| duanes_common::DuanesError::Serialization(e.to_string()))?;

        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    return Ok(None);
                }
                let chunk: duanes_common::models::StreamChunk = serde_json::from_str(data)
                    .map_err(|e| duanes_common::DuanesError::Serialization(e.to_string()))?;
                return Ok(Some(chunk));
            }
        }

        Ok(None)
    }

    pub fn count_prompt_tokens(request: &InferenceRequest) -> u64 {
        let total_chars: usize = request.messages.iter()
            .map(|m| m.content.text().len())
            .sum();
        (total_chars as u64) / 4
    }

    pub fn count_completion_tokens(text: &str) -> u64 {
        (text.len() as u64) / 4
    }
}

pub struct RequestTimer {
    start: Instant,
    first_token: Option<Instant>,
}

impl RequestTimer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            first_token: None,
        }
    }

    pub fn mark_first_token(&mut self) {
        if self.first_token.is_none() {
            self.first_token = Some(Instant::now());
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn time_to_first_token_ms(&self) -> Option<u64> {
        self.first_token.map(|t| t.duration_since(self.start).as_millis() as u64)
    }
}
