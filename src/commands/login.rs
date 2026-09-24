//! Interactive login with a Cloudflare API token (`cloudflare login`).

use std::io::{BufRead, IsTerminal, Write};

use crate::cli::Login;
use crate::client::Client;
use crate::commands::accounts::verify_token;
use crate::commands::print;
use crate::config::{self, AccountConfig};
use crate::error::{Error, Result};
use crate::obj;
use crate::secrets;

const API_TOKENS_URL: &str = "https://dash.cloudflare.com/profile/api-tokens";

pub fn run(args: Login) -> Result<()> {
    let Login { name, api_token, api_token_stdin, account_id, no_browser, force, no_verify } = args;

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
        return Err(Error::invalid(format!("An account profile named '{existing}' already exists."))
            .fix(format!("Use --force to replace its token: cloudflare login {existing} --force")));
    }
    let name = existing.clone().unwrap_or(name);

    let token = if api_token_stdin {
        read_stdin_token()?
    } else if let Some(k) = api_token.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
        k
    } else {
        prompt_login_token(&name, no_browser)?
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

    if std::io::stderr().is_terminal() {
        if !ident.is_empty() {
            eprintln!("Successfully logged in as {ident} to account '{name}'.");
        } else {
            eprintln!("Successfully logged in to account '{name}'.");
        }
    }

    print(obj! {
        "status" => "logged_in",
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

fn prompt_login_token(name: &str, no_browser: bool) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::invalid("No API token given and no terminal to prompt on.")
            .fix(format!("pbpaste | cloudflare login {name} --api-token-stdin")));
    }

    eprintln!("To log in, copy or create an API token from Cloudflare:");
    eprintln!("  {API_TOKENS_URL}\n");

    if !no_browser {
        eprintln!("Opening {API_TOKENS_URL} in your browser...");
        open_browser(API_TOKENS_URL);
    }

    let _ = std::io::stderr().flush();

    loop {
        let token = rpassword::prompt_password(format!("Paste your Cloudflare API token for '{name}': "))
            .map_err(|e| Error::other("Could not read API token.").detail(e.to_string()))?;
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
        eprintln!("Token cannot be empty. Paste your API token from {API_TOKENS_URL}");
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
