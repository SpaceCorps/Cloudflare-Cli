//! Account resolution and authentication context.
//!
//! Multi-account safety: accounts are stored under named profiles in the OS keystore.
//! For automation, `--api-token` or the `CLOUDFLARE_API_TOKEN` environment variable is also supported.

use serde_json::Value;

use crate::client::Client;
use crate::config::{self, AccountConfig, Config};
use crate::error::{Error, ErrorCode, Result};
use crate::secrets;

#[derive(Clone, Debug, Default)]
pub struct AuthArgs {
    pub account: Option<String>,
    pub api_token: Option<String>,
    pub account_id: Option<String>,
    pub endpoint: Option<String>,
}

pub struct Resolved {
    pub name: String,
    pub config: AccountConfig,
    pub api_token: String,
    pub endpoint: Option<String>,
}

impl Resolved {
    pub fn account_id(&self) -> Option<&str> {
        let id = self.config.account_id.as_str().trim();
        if id.is_empty() { None } else { Some(id) }
    }

    pub fn client(&self) -> Client {
        Client::new(&self.api_token, self.endpoint.as_deref())
    }
}

pub fn resolve(auth: &AuthArgs) -> Result<Resolved> {
    let token_arg =
        auth.api_token.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).or_else(|| {
            std::env::var("CLOUDFLARE_API_TOKEN").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        });

    let account_id_arg =
        auth.account_id.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).or_else(|| {
            std::env::var("CLOUDFLARE_ACCOUNT_ID").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        });

    let endpoint_arg = auth
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            std::env::var("CLOUDFLARE_API_ENDPOINT").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        })
        .or_else(|| std::env::var("CLOUDFLARE_API_URL").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));

    if let Some(token) = token_arg {
        let (name, mut account_cfg) =
            if let Some(acct_name) = auth.account.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                let config = config::load()?;
                if let Some((stored_name, cfg)) = config.find(acct_name) {
                    (stored_name.clone(), cfg.clone())
                } else {
                    (acct_name.to_string(), AccountConfig::default())
                }
            } else {
                ("inline".to_string(), AccountConfig::default())
            };

        if let Some(aid) = account_id_arg {
            account_cfg.account_id = aid;
        }

        return Ok(Resolved { name, config: account_cfg, api_token: token, endpoint: endpoint_arg });
    }

    let config = config::load()?;

    let Some(requested) = auth.account.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(Error::new(
            ErrorCode::NoAccount,
            "No account specified. Pass --account <name>, --api-token <token>, or set CLOUDFLARE_API_TOKEN.",
        )
        .detail(describe(&config))
        .fix("cloudflare accounts list"));
    };

    let Some((name, account)) = config.find(requested) else {
        return Err(Error::new(ErrorCode::NoAccount, format!("No account named '{requested}'."))
            .detail(describe(&config))
            .fix("cloudflare accounts list"));
    };

    let key = secrets::store()?.get(&secrets::account_key(name))?;

    let Some(api_token) = key.filter(|k| !k.trim().is_empty()) else {
        return Err(Error::new(ErrorCode::AuthRequired, format!("Account '{name}' has no stored API token."))
            .detail("The config entry exists but the keystore has nothing under it.")
            .fix(format!("cloudflare accounts add {name} --api-token <token>")));
    };

    let mut account_cfg = account.clone();
    if let Some(aid) = account_id_arg {
        account_cfg.account_id = aid;
    }

    Ok(Resolved { name: name.clone(), config: account_cfg, api_token, endpoint: endpoint_arg })
}

fn describe(config: &Config) -> String {
    if config.accounts.is_empty() {
        return "No accounts are configured yet. Run 'cloudflare accounts add <name> --api-token <token>'.".into();
    }
    let listed: Vec<String> = config
        .sorted()
        .into_iter()
        .map(|(k, v)| if v.identity.trim().is_empty() { k.clone() } else { format!("{k} ({})", v.identity) })
        .collect();
    format!("Configured accounts: {}", listed.join(", "))
}

pub mod identity {
    use super::Value;

    pub fn describe(verify_resp: &Value) -> String {
        if let Some(id) = verify_resp.get("id").and_then(Value::as_str) {
            let status = verify_resp.get("status").and_then(Value::as_str).unwrap_or("active");
            return format!("{id} ({status})");
        }
        if let Some(email) = verify_resp.get("email").and_then(Value::as_str) {
            return email.to_string();
        }
        String::new()
    }
}
