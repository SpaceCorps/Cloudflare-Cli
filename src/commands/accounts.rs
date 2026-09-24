//! Account profile management (`cloudflare accounts add|list|test|remove`).

use std::io::{BufRead, IsTerminal, Write};

use serde_json::Value;

use crate::account::{self, identity};
use crate::cli::Accounts;
use crate::client::Client;
use crate::commands::print;
use crate::config::{self, AccountConfig};
use crate::error::{Error, ErrorCode, Result};
use crate::obj;
use crate::secrets::{self, Store};

pub fn run(c: Accounts) -> Result<()> {
    match c {
        Accounts::Add { name, api_token, api_token_stdin, account_id, force, no_verify } => {
            let token = if api_token_stdin { Some(read_stdin_token()?) } else { api_token };
            add(name, token, account_id, force, no_verify)
        }
        Accounts::List { check } => list(check),
        Accounts::Test { name } => test(&name),
        Accounts::Remove { name, yes } => remove(&name, yes),
    }
}

pub(crate) fn verify_token(client: &Client) -> Result<String> {
    // Attempt standard Cloudflare token verify endpoint
    if let Ok(resp) = client.get("user/tokens/verify") {
        let ident = identity::describe(&resp);
        if !ident.is_empty() {
            return Ok(ident);
        }
    }
    // Fallback: test if zones can be queried
    if client.get("zones?per_page=1").is_ok() {
        return Ok("active (zone permissions)".to_string());
    }
    // Final check: user info
    if let Ok(resp) = client.get("user") {
        let ident = identity::describe(&resp);
        if !ident.is_empty() {
            return Ok(ident);
        }
    }

    Err(Error::new(ErrorCode::AuthRequired, "The Cloudflare API token could not be verified.")
        .fix("Check that the token is valid and active at https://dash.cloudflare.com/profile/api-tokens"))
}

fn add(
    name: String,
    api_token: Option<String>,
    account_id: Option<String>,
    force: bool,
    no_verify: bool,
) -> Result<()> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::invalid("An account profile name is required."));
    }

    let store = secrets::store()?;
    let config = config::load()?;

    let existing = config.find(&name).map(|(k, _)| k.clone());
    if let Some(existing) = &existing
        && !force
    {
        return Err(Error::invalid(format!("An account profile named '{existing}' already exists.")).fix(format!(
            "Pick a different name, or replace its token: cloudflare accounts add {existing} --api-token <token> --force"
        )));
    }
    let name = existing.clone().unwrap_or(name);

    let token = match api_token.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
        Some(k) => k,
        None => prompt_token(&name)?,
    };
    let account_id = account_id.map(|o| o.trim().to_string()).unwrap_or_default();

    let mut ident = String::new();
    if !no_verify {
        let client = Client::new(&token, None);
        ident = verify_token(&client)?;
    }

    {
        let _lock = config::lock()?;
        store.set(&secrets::account_key(&name), &token)?;

        let mut config = config::load()?;
        config.accounts.insert(
            name.clone(),
            AccountConfig {
                account_id: account_id.clone(),
                identity: ident.clone(),
                account_name: String::new(),
                added_at: config::now_utc(),
            },
        );
        config::save(&config)?;
    }

    print(obj! {
        "status" => if existing.is_none() { "added" } else { "replaced" },
        "name" => name,
        "identity" => ident,
        "accountId" => account_id,
        "verified" => !no_verify,
        "secretStore" => store.name(),
        "configDir" => config::config_dir().display().to_string(),
        "nextStep" => format!("cloudflare zones list --account {name}"),
    })
}

fn read_stdin_token() -> Result<String> {
    let mut token = String::new();
    std::io::stdin().lock().read_line(&mut token).map_err(|e| Error::invalid(format!("Could not read stdin: {e}")))?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(Error::invalid("--api-token-stdin was given but stdin was empty."));
    }
    Ok(token)
}

fn prompt_token(name: &str) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::invalid("No API token given and no terminal to prompt on.")
            .fix(format!("pbpaste | cloudflare accounts add {name} --api-token-stdin")));
    }
    loop {
        let token = rpassword::prompt_password(format!("Cloudflare API token for {name}: "))
            .map_err(|e| Error::other("Could not read API token.").detail(e.to_string()))?;
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
        eprintln!("Cannot be empty");
    }
}

fn list(check: bool) -> Result<()> {
    let store = secrets::store()?;
    let config = config::load()?;
    let sorted = config.sorted();

    let statuses: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> =
            sorted.iter().map(|(name, _)| scope.spawn(move || status_of(name, store, check))).collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| "unreachable".into())).collect()
    });

    let accounts: Vec<Value> = sorted
        .iter()
        .zip(statuses)
        .map(|((name, a), status)| {
            obj! {
                "name" => name,
                "identity" => a.identity,
                "accountId" => a.account_id,
                "addedAt" => a.added_at,
                "tokenStatus" => status,
            }
        })
        .collect();

    print(obj! {
        "count" => accounts.len(),
        "accounts" => accounts,
        "secretStore" => store.name(),
        "configDir" => config::config_dir().display().to_string(),
    })
}

fn status_of(name: &str, store: Store, check: bool) -> String {
    let token = match store.get(&secrets::account_key(name)) {
        Ok(Some(k)) if !k.trim().is_empty() => k,
        Ok(_) => return "missing_token".into(),
        Err(_) => return "unreadable".into(),
    };
    if !check {
        return "stored".into();
    }
    let client = Client::new(&token, None);
    match verify_token(&client) {
        Ok(_) => "valid".into(),
        Err(e) if e.code == ErrorCode::AuthRequired => "rejected".into(),
        Err(_) => "unreachable".into(),
    }
}

fn test(name: &str) -> Result<()> {
    let auth = account::AuthArgs { account: Some(name.to_string()), ..Default::default() };
    let resolved = account::resolve(&auth)?;
    let client = resolved.client();
    let ident = verify_token(&client)?;

    let result = obj! {
        "name" => resolved.name,
        "identity" => ident,
        "accountId" => resolved.config.account_id,
        "tokenStatus" => "valid",
    };

    print(result)
}

fn remove(requested: &str, yes: bool) -> Result<()> {
    let store = secrets::store()?;
    let config = config::load()?;

    let Some((name, acct)) = config.find(requested) else {
        return Err(Error::new(ErrorCode::NoAccount, format!("No account named '{requested}'."))
            .fix("cloudflare accounts list"));
    };
    let (name, acct) = (name.clone(), acct.clone());

    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err(Error::invalid(format!(
                "Removing '{name}' needs confirmation and there is no terminal to ask on."
            ))
            .fix(format!("cloudflare accounts remove {name} --yes")));
        }
        let label = if acct.identity.trim().is_empty() { name.clone() } else { format!("{name} ({})", acct.identity) };
        eprint!("Remove account {label}? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        let _ = std::io::stdin().lock().read_line(&mut answer);
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            return Err(Error::invalid("Cancelled."));
        }
    }

    {
        let _lock = config::lock()?;
        store.delete(&secrets::account_key(&name))?;
        let mut config = config::load()?;
        config.accounts.shift_remove(&name);
        config::save(&config)?;
    }

    print(obj! {
        "status" => "removed",
        "name" => name,
        "identity" => acct.identity,
        "note" => "The API token was deleted locally. Revoke it at https://dash.cloudflare.com/profile/api-tokens if it should stop working everywhere.",
    })
}
