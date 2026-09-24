//! Commands for reading and writing zone settings (`cloudflare settings list|get|set`).

use serde_json::json;

use crate::cli::Settings;
use crate::commands::{print, resolve_zone};
use crate::error::Result;

pub fn run(cmd: Settings) -> Result<()> {
    match cmd {
        Settings::List(zone_auth) => {
            let (client, zone_id) = resolve_zone(&zone_auth)?;
            print(client.get(&format!("zones/{zone_id}/settings"))?)
        }
        Settings::Get { zone, name } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            print(client.get(&format!("zones/{zone_id}/settings/{name}"))?)
        }
        Settings::Set { zone, name, value } => {
            let (client, zone_id) = resolve_zone(&zone)?;
            let res = client.patch(&format!("zones/{zone_id}/settings/{name}"), &json!({ "value": value }))?;
            print(res)
        }
    }
}
