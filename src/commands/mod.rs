//! Command dispatch and output helpers.

pub mod accounts;
pub mod login;
pub mod pagerules;
pub mod records;
pub mod settings;
pub mod ssl;
pub mod zones;

use serde_json::Value;

use crate::account::{self, Resolved};
use crate::cli::*;
use crate::client::Client;
use crate::error::Result;
use crate::{obj, output};

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Login(c) => login::run(c),
        Command::AgentReadme => {
            crate::readme::print();
            Ok(())
        }
        Command::Accounts(c) => accounts::run(c),
        Command::Zones(c) => zones::run(c),
        Command::Records(c) => records::run(c),
        Command::Settings(c) => settings::run(c),
        Command::Ssl(c) => ssl::run(c),
        Command::Pagerules(c) => pagerules::run(c),
    }
}

pub(crate) fn resolve_auth(auth: &Auth) -> Result<Resolved> {
    account::resolve(&auth.to_auth_args())
}

pub(crate) fn client(auth: &Auth) -> Result<Client> {
    Ok(resolve_auth(auth)?.client())
}

pub(crate) fn resolve_zone(zone_auth: &ZoneAuth) -> Result<(Client, String)> {
    let client = client(&zone_auth.auth)?;
    let zone_id = client.resolve_zone_id(&zone_auth.zone)?;
    Ok((client, zone_id))
}

pub(crate) fn print(v: Value) -> Result<()> {
    output::write(&v);
    Ok(())
}

/// For endpoints that answer with empty or simple objects: ensures stdout is never blank and parseable.
pub(crate) fn done(v: Value, status: &str, fields: Value) -> Result<()> {
    let mut out = obj! { "status" => status };
    if let Value::Object(o) = &mut out {
        if let Value::Object(f) = fields {
            o.extend(f);
        }
        if let Value::Object(m) = v {
            o.extend(m);
        }
    }
    print(out)
}
