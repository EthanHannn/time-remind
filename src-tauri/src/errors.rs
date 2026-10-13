//! Stable IPC error envelopes. Technical errors remain in logs; UI translates codes.
pub fn code(code: &str) -> String {
    params(code, serde_json::json!({}))
}

pub fn params(code: &str, params: serde_json::Value) -> String {
    format!(
        "time-remind:{}",
        serde_json::json!({ "code": code, "params": params })
    )
}
