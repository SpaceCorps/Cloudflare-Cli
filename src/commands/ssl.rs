//! Commands for inspecting SSL/TLS configuration (`cloudflare ssl status`).

use serde_json::Value;

use crate::cli::Ssl;
use crate::client::Client;
use crate::commands::{print, resolve_zone};
use crate::error::Result;
use crate::obj;

pub fn run(cmd: Ssl) -> Result<()> {
    match cmd {
        Ssl::Status(zone_auth) => {
            let (client, zone_id) = resolve_zone(&zone_auth)?;

            let mode = safe_get(&client, &format!("zones/{zone_id}/settings/ssl"));
            let always_https = safe_get(&client, &format!("zones/{zone_id}/settings/always_use_https"));
            let universal = safe_get(&client, &format!("zones/{zone_id}/ssl/universal/settings"));
            let packs = safe_get(&client, &format!("zones/{zone_id}/ssl/certificate_packs?status=all"));

            let encryption_mode = extract_value(&mode);
            let always_use_https = extract_value(&always_https);
            let universal_ssl_enabled =
                universal.as_object().and_then(|o| o.get("enabled")).cloned().unwrap_or(universal);
            let cert_packs = summarize_packs(&packs);

            let out = obj! {
                "encryption_mode" => encryption_mode,
                "always_use_https" => always_use_https,
                "universal_ssl_enabled" => universal_ssl_enabled,
                "certificate_packs" => cert_packs,
            };

            print(out)
        }
    }
}

fn safe_get(client: &Client, path: &str) -> Value {
    match client.get(path) {
        Ok(v) => v,
        Err(e) => Value::String(format!("unavailable: {}", e.message)),
    }
}

fn extract_value(setting: &Value) -> Value {
    if let Some(obj) = setting.as_object()
        && let Some(v) = obj.get("value")
    {
        return v.clone();
    }
    setting.clone()
}

fn summarize_packs(packs: &Value) -> Value {
    if let Some(list) = packs.as_array() {
        let summarized: Vec<Value> = list
            .iter()
            .filter_map(|p| p.as_object())
            .map(|p| {
                obj! {
                    "type" => p.get("type").unwrap_or(&Value::Null),
                    "status" => p.get("status").unwrap_or(&Value::Null),
                    "hosts" => p.get("hosts").unwrap_or(&Value::Null),
                    "certificate_authority" => p.get("certificate_authority").unwrap_or(&Value::Null),
                }
            })
            .collect();
        return Value::Array(summarized);
    }
    packs.clone()
}
