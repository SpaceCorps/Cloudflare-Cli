//! Commands for managing Cloudflare zones (`cloudflare zones list|get|add|purge-cache`).

use serde_json::json;

use crate::cli::Zones;
use crate::commands::{client, done, print, resolve_auth, resolve_zone};
use crate::error::{Error, Result};
use crate::obj;

pub fn run(cmd: Zones) -> Result<()> {
    match cmd {
        Zones::List(auth) => {
            let client = client(&auth)?;
            print(client.get("zones")?)
        }
        Zones::Get(zone_auth) => {
            let (client, zone_id) = resolve_zone(&zone_auth)?;
            print(client.get(&format!("zones/{zone_id}"))?)
        }
        Zones::Add { name, jump_start, auth } => {
            let resolved = resolve_auth(&auth)?;
            let client = resolved.client();
            let aid = resolved.account_id().filter(|s| !s.trim().is_empty()).ok_or_else(|| {
                Error::invalid("Account ID is required for 'zones add'.")
                    .detail("Zone creation requires an account ID.")
                    .fix(
                        "Pass --account-id <id>, set CLOUDFLARE_ACCOUNT_ID, or store it via 'cloudflare accounts add'.",
                    )
            })?;

            let body = json!({
                "name": name,
                "account": { "id": aid },
                "jump_start": jump_start,
            });

            let res = client.post("zones", &body)?;
            print(res)
        }
        Zones::PurgeCache { zone, everything, files } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            let body = if everything {
                json!({ "purge_everything": true })
            } else if let Some(urls) = files.filter(|f| !f.is_empty()) {
                json!({ "files": urls })
            } else {
                return Err(Error::invalid(
                    "Specify --everything to purge all cached assets, or --file <URL> to purge specific files.",
                ));
            };

            let res = client.post(&format!("zones/{zone_id}/purge_cache"), &body)?;
            done(res, "purged", obj! { "zoneId" => zone_id })
        }
    }
}
