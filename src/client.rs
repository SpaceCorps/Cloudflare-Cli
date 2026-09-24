//! HTTP client for the Cloudflare API v4 and error translation.

use std::time::Duration;

use serde_json::Value;
use ureq::Agent;
use ureq::http::Response;

use crate::error::{Error, ErrorCode, Result};

const DEFAULT_BASE: &str = "https://api.cloudflare.com/client/v4/";
const MAX_BODY: u64 = 512 * 1024 * 1024;

#[derive(Clone)]
pub struct Client {
    agent: Agent,
    base: String,
    auth: String,
}

#[allow(dead_code)]
enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Client {
    pub fn new(api_token: &str, endpoint: Option<&str>) -> Client {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(100)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .http_status_as_error(false)
            .user_agent(concat!("cloudflare-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();

        let mut base = endpoint
            .map(str::to_string)
            .or_else(|| std::env::var("CLOUDFLARE_API_URL").ok())
            .or_else(|| std::env::var("CLOUDFLARE_API_ENDPOINT").ok())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE.to_string());
        if !base.ends_with('/') {
            base.push('/');
        }

        Client { agent, base, auth: format!("Bearer {api_token}") }
    }

    pub fn get(&self, path: &str) -> Result<Value> {
        self.send(Method::Get, path, None)
    }

    #[allow(dead_code)]
    pub fn post_empty(&self, path: &str) -> Result<Value> {
        self.send(Method::Post, path, None)
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(Method::Post, path, Some(body))
    }

    #[allow(dead_code)]
    pub fn put(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(Method::Put, path, Some(body))
    }

    pub fn patch(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(Method::Patch, path, Some(body))
    }

    pub fn delete(&self, path: &str) -> Result<Value> {
        self.send(Method::Delete, path, None)
    }

    /// Returns raw response body as string, for endpoints like BIND export that do not return JSON.
    pub fn get_raw(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.base, path.trim_start_matches('/'));
        let response = self
            .agent
            .get(&url)
            .header("Authorization", &self.auth)
            .header("Accept", "text/plain, application/json, */*")
            .call()
            .map_err(transport_error)?;

        let status = response.status().as_u16();
        let bytes = response.into_body().with_config().limit(MAX_BODY).read_to_vec().map_err(transport_error)?;
        let text = String::from_utf8_lossy(&bytes).to_string();

        if !(200..300).contains(&status) {
            return Err(status_error(status, &text));
        }

        Ok(text)
    }

    /// Multipart form upload for BIND import.
    pub fn post_multipart(&self, path: &str, file_content: &str, fields: &[(&str, &str)]) -> Result<Value> {
        let url = format!("{}{}", self.base, path.trim_start_matches('/'));
        let boundary = "----CloudflareCliBoundary47284910283";

        let mut body = Vec::new();
        // File part
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"bind_config.txt\"\r\n");
        body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
        body.extend_from_slice(file_content.as_bytes());
        body.extend_from_slice(b"\r\n");

        // Fields
        for (k, v) in fields {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{k}\"\r\n\r\n").as_bytes());
            body.extend_from_slice(v.as_bytes());
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let content_type = format!("multipart/form-data; boundary={boundary}");
        let response = self
            .agent
            .post(&url)
            .header("Authorization", &self.auth)
            .header("Accept", "application/json")
            .header("Content-Type", &content_type)
            .send(&body[..])
            .map_err(transport_error)?;

        read(response)
    }

    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let url = format!("{}{}", self.base, path.trim_start_matches('/'));

        macro_rules! headers {
            ($req:expr) => {{ $req.header("Authorization", &self.auth).header("Accept", "application/json") }};
        }

        let result = match (method, body) {
            (Method::Get, _) => headers!(self.agent.get(&url)).call(),
            (Method::Delete, _) => headers!(self.agent.delete(&url)).call(),
            (m, Some(b)) => {
                let json = serde_json::to_vec(b).expect("a Value always serializes");
                let req = match m {
                    Method::Post => self.agent.post(&url),
                    Method::Put => self.agent.put(&url),
                    _ => self.agent.patch(&url),
                };
                headers!(req).header("Content-Type", "application/json").send(&json[..])
            }
            (m, None) => {
                let req = match m {
                    Method::Post => self.agent.post(&url),
                    Method::Put => self.agent.put(&url),
                    _ => self.agent.patch(&url),
                };
                headers!(req).send_empty()
            }
        };

        let response = result.map_err(transport_error)?;
        read(response)
    }

    /// Resolves either a 32-hex zone ID or a zone domain name into a zone ID.
    pub fn resolve_zone_id(&self, zone: &str) -> Result<String> {
        let zone = zone.trim();
        if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(zone.to_string());
        }

        let res = self.get(&format!("zones?name={}", seg(zone)))?;
        if let Some(list) = res.as_array()
            && let Some(first) = list.first()
            && let Some(id) = first.get("id").and_then(Value::as_str)
        {
            return Ok(id.to_string());
        }

        Err(Error::new(ErrorCode::NotFound, format!("No zone found named '{zone}'."))
            .detail("Zone lookup by domain name returned no matches.")
            .fix("Check the zone name with 'cloudflare zones list'."))
    }

    /// Resolves a DNS record ID by name, optionally filtered by type.
    pub fn resolve_record_id(&self, zone_id: &str, name: &str, record_type: Option<&str>) -> Result<String> {
        let mut query_path = format!("zones/{zone_id}/dns_records?name={}", seg(name));
        if let Some(t) = record_type.filter(|s| !s.trim().is_empty()) {
            query_path.push_str(&format!("&type={}", seg(t)));
        }

        let res = self.get(&query_path)?;
        if let Some(list) = res.as_array() {
            if list.is_empty() {
                return Err(Error::new(ErrorCode::NotFound, format!("No DNS record found named '{name}'."))
                    .fix(format!("cloudflare records list --zone {zone_id}")));
            }
            if list.len() > 1 {
                return Err(Error::new(
                    ErrorCode::InvalidInput,
                    format!("'{name}' matches {} records. Narrow it with --type, or use --id.", list.len()),
                ));
            }
            if let Some(id) = list[0].get("id").and_then(Value::as_str) {
                return Ok(id.to_string());
            }
        }

        Err(Error::new(ErrorCode::NotFound, format!("No DNS record found named '{name}'.")))
    }
}

fn read(mut response: Response<ureq::Body>) -> Result<Value> {
    let status = response.status().as_u16();
    let bytes = response.body_mut().with_config().limit(MAX_BODY).read_to_vec().map_err(transport_error)?;

    if bytes.iter().all(u8::is_ascii_whitespace) {
        if (200..300).contains(&status) {
            return Ok(Value::Object(Default::default()));
        }
        return Err(status_error(status, ""));
    }

    let parsed: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            let text = String::from_utf8_lossy(&bytes).to_string();
            if !(200..300).contains(&status) {
                return Err(status_error(status, &text));
            }
            return Ok(Value::String(text));
        }
    };

    // Check Cloudflare standard response envelope: { success: bool, errors: [...], result: ... }
    if let Some(obj) = parsed.as_object() {
        if let Some(success) = obj.get("success").and_then(Value::as_bool)
            && !success
        {
            let mut error_messages = Vec::new();
            let mut first_code = None;
            if let Some(errors) = obj.get("errors").and_then(Value::as_array) {
                for err in errors {
                    let code_str = err.get("code").map(|c| c.to_string()).unwrap_or_else(|| "?".into());
                    if first_code.is_none() {
                        first_code = err.get("code").and_then(Value::as_u64);
                    }
                    let msg = err.get("message").and_then(Value::as_str).unwrap_or("");
                    error_messages.push(format!("[{code_str}] {msg}"));
                }
            }
            let details = if error_messages.is_empty() {
                serde_json::to_string(&parsed).unwrap_or_default()
            } else {
                error_messages.join("; ")
            };

            let err_code = match (status, first_code) {
                (401 | 403, _) | (_, Some(10000 | 10001)) => ErrorCode::AuthRequired,
                (404, _) | (_, Some(7000 | 7003)) => ErrorCode::NotFound,
                (429, _) | (_, Some(10013)) => ErrorCode::RateLimited,
                (400 | 422, _) | (_, Some(6007 | 1004)) => ErrorCode::InvalidInput,
                (s, _) if s >= 500 => ErrorCode::Network,
                _ => ErrorCode::Error,
            };

            let mut err = Error::new(err_code, format!("Cloudflare API error: {details}"))
                .detail(format!("HTTP {status}: {details}"));
            if err_code == ErrorCode::AuthRequired {
                err = err.fix("Check your API token and permissions: https://dash.cloudflare.com/profile/api-tokens");
            }
            return Err(err);
        }

        if !(200..300).contains(&status) {
            let body = serde_json::to_string(&parsed).unwrap_or_default();
            return Err(status_error(status, &body));
        }

        // Return the unwrapped 'result' field if present and not null
        if let Some(result) = obj.get("result")
            && !result.is_null()
        {
            return Ok(result.clone());
        }
    }

    if !(200..300).contains(&status) {
        let body = serde_json::to_string(&parsed).unwrap_or_default();
        return Err(status_error(status, &body));
    }

    Ok(parsed)
}

fn transport_error(e: ureq::Error) -> Error {
    match e {
        ureq::Error::Timeout(_) => {
            Error::new(ErrorCode::Network, "The request timed out.").fix("Retry once, then stop.")
        }
        other => Error::new(ErrorCode::Network, "Could not reach the Cloudflare API.")
            .detail(other.to_string())
            .fix("Check network connection and endpoint, retry once."),
    }
}

pub fn status_error(status: u16, body: &str) -> Error {
    let mut detail = format!("HTTP {status}");
    if !body.is_empty() {
        detail.push_str(": ");
        detail.push_str(body);
    }

    let e = match status {
        401 | 403 => Error::new(ErrorCode::AuthRequired, "Cloudflare API token was rejected or missing permissions.")
            .fix("Verify token permissions at https://dash.cloudflare.com/profile/api-tokens"),
        404 => Error::new(ErrorCode::NotFound, "The requested resource was not found on Cloudflare."),
        429 => Error::new(ErrorCode::RateLimited, "Rate limited by Cloudflare API.").fix("Back off before retrying."),
        400 | 422 => Error::new(ErrorCode::InvalidInput, "The API refused the request."),
        s if s >= 500 => Error::new(ErrorCode::Network, "Cloudflare API returned a server error.")
            .fix("Retry; if it persists, check https://www.cloudflarestatus.com"),
        _ => Error::new(ErrorCode::Error, "The request failed."),
    };
    e.detail(detail)
}

/// Percent-encodes one path or query segment.
pub fn seg(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Builds `?a=1&b=2` from present query parameter pairs.
#[allow(dead_code)]
pub fn query(pairs: &[(&str, Option<String>)]) -> String {
    let parts: Vec<String> = pairs.iter().filter_map(|(k, v)| v.as_ref().map(|v| format!("{k}={}", seg(v)))).collect();
    if parts.is_empty() { String::new() } else { format!("?{}", parts.join("&")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_escapes_reserved() {
        assert_eq!(seg("example.com"), "example.com");
        assert_eq!(seg("A B/C"), "A%20B%2FC");
    }

    #[test]
    fn query_builds_pairs() {
        assert_eq!(query(&[("a", None), ("type", Some("A".into()))]), "?type=A");
        assert_eq!(query(&[("a", None)]), "");
    }
}
