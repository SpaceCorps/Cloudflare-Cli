//! Commands for managing DNS records (`cloudflare records list|export|import|add|update|delete|proxy`).

use serde_json::json;

use crate::cli::Records;
use crate::client::seg;
use crate::commands::{done, print, resolve_zone};
use crate::error::{Error, Result};
use crate::{obj, output};

pub fn run(cmd: Records) -> Result<()> {
    match cmd {
        Records::List { zone, record_type, name, per_page } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            let mut q = format!("zones/{zone_id}/dns_records?per_page={per_page}");
            if let Some(t) = record_type.filter(|s| !s.trim().is_empty()) {
                q.push_str(&format!("&type={}", seg(&t)));
            }
            if let Some(n) = name.filter(|s| !s.trim().is_empty()) {
                q.push_str(&format!("&name={}", seg(&n)));
            }
            print(client.get(&q)?)
        }
        Records::Export { zone, out } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            let text = client.get_raw(&format!("zones/{zone_id}/dns_records/export"))?;

            if let Some(path) = out {
                std::fs::write(&path, &text)
                    .map_err(|e| Error::invalid(format!("Could not write to '{path}': {e}")))?;
                done(json!(null), "exported", obj! { "file" => path, "zoneId" => zone_id })
            } else if output::json() {
                print(obj! { "export" => text, "zoneId" => zone_id })
            } else {
                print!("{text}");
                Ok(())
            }
        }
        Records::Import { zone, file, proxied } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            let content =
                std::fs::read_to_string(&file).map_err(|e| Error::invalid(format!("Could not read '{file}': {e}")))?;

            let res = client.post_multipart(
                &format!("zones/{zone_id}/dns_records/import"),
                &content,
                &[("proxied", if proxied { "true" } else { "false" })],
            )?;
            print(res)
        }
        Records::Add { zone, record_type, name, content, ttl, priority, proxied } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            let mut body = obj! {
                "type" => record_type.to_uppercase(),
                "name" => name,
                "content" => content,
                "ttl" => ttl,
                "proxied" => proxied,
            };
            if let Some(p) = priority {
                body["priority"] = json!(p);
            }

            let res = client.post(&format!("zones/{zone_id}/dns_records"), &body)?;
            print(res)
        }
        Records::Update { zone, id, name, record_type, content, ttl, proxied } => {
            if id.is_none() && name.is_none() {
                return Err(Error::invalid("Either --id or --name is required."));
            }

            let (client, zone_id) = resolve_zone(&zone)?;
            let record_id = if let Some(i) = id {
                i
            } else {
                client.resolve_record_id(&zone_id, name.as_deref().unwrap(), record_type.as_deref())?
            };

            let mut body = obj! {};
            if let Some(t) = record_type {
                body["type"] = json!(t.to_uppercase());
            }
            if let Some(n) = name {
                body["name"] = json!(n);
            }
            if let Some(c) = content {
                body["content"] = json!(c);
            }
            if let Some(t) = ttl {
                body["ttl"] = json!(t);
            }
            if let Some(p) = proxied {
                body["proxied"] = json!(p);
            }

            let res = client.patch(&format!("zones/{zone_id}/dns_records/{record_id}"), &body)?;
            print(res)
        }
        Records::Delete { zone, id, name, record_type } => {
            if id.is_none() && name.is_none() {
                return Err(Error::invalid("Either --id or --name is required."));
            }

            let (client, zone_id) = resolve_zone(&zone)?;
            let record_id = if let Some(i) = id {
                i
            } else {
                client.resolve_record_id(&zone_id, name.as_deref().unwrap(), record_type.as_deref())?
            };

            let res = client.delete(&format!("zones/{zone_id}/dns_records/{record_id}"))?;
            done(res, "deleted", obj! { "zoneId" => zone_id, "recordId" => record_id })
        }
        Records::Proxy { zone, name, record_type, on, off } => {
            if on == off {
                return Err(Error::invalid("Specify exactly one of --on or --off."));
            }

            let (client, zone_id) = resolve_zone(&zone)?;
            let record_id = client.resolve_record_id(&zone_id, &name, record_type.as_deref())?;

            let res = client.patch(&format!("zones/{zone_id}/dns_records/{record_id}"), &json!({ "proxied": on }))?;
            print(res)
        }
    }
}
