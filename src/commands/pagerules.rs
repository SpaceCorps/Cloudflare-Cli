//! Commands for managing Cloudflare page rules (`cloudflare pagerules list|add`).

use serde_json::json;

use crate::cli::PageRules;
use crate::commands::{print, resolve_zone};
use crate::error::{Error, Result};

pub fn run(cmd: PageRules) -> Result<()> {
    match cmd {
        PageRules::List(zone_auth) => {
            let (client, zone_id) = resolve_zone(&zone_auth)?;
            print(client.get(&format!("zones/{zone_id}/pagerules"))?)
        }
        PageRules::Add { zone, url, always_use_https, priority, disabled } => {
            if !always_use_https {
                return Err(Error::invalid("No action specified. Pass --always-use-https."));
            }

            let (client, zone_id) = resolve_zone(&zone)?;
            let body = json!({
                "targets": [
                    {
                        "target": "url",
                        "constraint": { "operator": "matches", "value": url }
                    }
                ],
                "actions": [
                    { "id": "always_use_https" }
                ],
                "priority": priority,
                "status": if disabled { "disabled" } else { "active" },
            });

            let res = client.post(&format!("zones/{zone_id}/pagerules"), &body)?;
            print(res)
        }
    }
}
