//! Command-line interface definitions and clap hierarchy.

use clap::{Args, Parser, Subcommand};

use crate::account::AuthArgs;

#[derive(Parser)]
#[command(
    name = "cloudflare",
    version,
    about = "CLI for the Cloudflare API - manage zones, DNS records, zone settings, SSL status, and page rules",
    after_help = "An LLM agent should start with: cloudflare agent-readme",
    propagate_version = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Print raw JSON instead of YAML, for scripting
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Clone, Debug, Default)]
pub struct Auth {
    /// Account profile to run against (see 'cloudflare accounts list')
    #[arg(short = 'a', long, value_name = "ACCOUNT", global = true)]
    pub account: Option<String>,

    /// Cloudflare API token (or set CLOUDFLARE_API_TOKEN env var)
    #[arg(long, value_name = "TOKEN", global = true)]
    pub api_token: Option<String>,

    /// Cloudflare account ID (or set CLOUDFLARE_ACCOUNT_ID env var)
    #[arg(long, value_name = "ID", global = true)]
    pub account_id: Option<String>,

    /// API base URL (or set CLOUDFLARE_API_ENDPOINT env var)
    #[arg(long, value_name = "URL", global = true)]
    pub endpoint: Option<String>,
}

impl Auth {
    pub fn to_auth_args(&self) -> AuthArgs {
        AuthArgs {
            account: self.account.clone(),
            api_token: self.api_token.clone(),
            account_id: self.account_id.clone(),
            endpoint: self.endpoint.clone(),
        }
    }
}

#[derive(Args, Clone, Debug)]
pub struct ZoneAuth {
    /// Zone name (e.g. example.com) or 32-character zone ID
    #[arg(long, value_name = "ZONE")]
    pub zone: String,

    #[command(flatten)]
    pub auth: Auth,
}

#[derive(Subcommand)]
pub enum Command {
    /// Log in with a Cloudflare API token
    Login(Login),
    /// Print the operating manual for an LLM agent driving this CLI
    AgentReadme,
    /// Manage Cloudflare account profiles and credentials
    #[command(subcommand)]
    Accounts(Accounts),
    /// Manage zones
    #[command(subcommand)]
    Zones(Zones),
    /// Manage DNS records
    #[command(subcommand)]
    Records(Records),
    /// Read and write zone settings
    #[command(subcommand)]
    Settings(Settings),
    /// Inspect TLS configuration
    #[command(subcommand)]
    Ssl(Ssl),
    /// Manage page rules
    #[command(subcommand)]
    Pagerules(PageRules),
}

// ---------------------------------------------------------------------------------------------
// login

#[derive(Args, Clone)]
pub struct Login {
    /// Account name to store (default: "default")
    #[arg(value_name = "NAME", default_value = "default")]
    pub name: String,

    /// Cloudflare API token (prompted for securely if omitted)
    #[arg(long, value_name = "TOKEN", conflicts_with = "api_token_stdin")]
    pub api_token: Option<String>,

    /// Read the API token from stdin, e.g. `pbpaste | cloudflare login --api-token-stdin`
    #[arg(long)]
    pub api_token_stdin: bool,

    /// Cloudflare account ID (used for 'zones add' and account-scoped APIs)
    #[arg(long, value_name = "ID")]
    pub account_id: Option<String>,

    /// Do not open the browser to the API tokens page automatically
    #[arg(long)]
    pub no_browser: bool,

    /// Replace the token on an account that already exists
    #[arg(long)]
    pub force: bool,

    /// Store the token without calling the API to verify it first
    #[arg(long)]
    pub no_verify: bool,
}

// ---------------------------------------------------------------------------------------------
// accounts

#[derive(Subcommand)]
pub enum Accounts {
    /// Add an account profile and store its API token in the OS keystore
    Add {
        /// Short name for this account profile, used as --account elsewhere
        name: String,
        /// Cloudflare API token (prompted for securely if omitted)
        #[arg(long, value_name = "TOKEN", conflicts_with = "api_token_stdin")]
        api_token: Option<String>,
        /// Read the API token from stdin
        #[arg(long)]
        api_token_stdin: bool,
        /// Cloudflare account ID (used for 'zones add')
        #[arg(long, value_name = "ID")]
        account_id: Option<String>,
        /// Replace the token on an account profile that already exists
        #[arg(long)]
        force: bool,
        /// Store the token without calling the API to verify it first
        #[arg(long)]
        no_verify: bool,
    },
    /// List configured account profiles
    List {
        /// Call the API once per account instead of reporting stored state
        #[arg(long)]
        check: bool,
    },
    /// Check that an account's stored token is valid
    Test {
        /// Account profile name
        name: String,
    },
    /// Remove an account profile and delete its stored token
    Remove {
        /// Account profile name
        name: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

// ---------------------------------------------------------------------------------------------
// zones

#[derive(Subcommand)]
pub enum Zones {
    /// List all zones in the account
    List(Auth),
    /// Get a zone, including its status and assigned name servers
    Get(ZoneAuth),
    /// Add a domain as a new zone
    Add {
        /// The domain to add, e.g. example.com
        #[arg(long, value_name = "DOMAIN")]
        name: String,
        /// Let Cloudflare scan for existing DNS records
        #[arg(long)]
        jump_start: bool,
        #[command(flatten)]
        auth: Auth,
    },
    /// Purge cached resources from Cloudflare edge servers
    PurgeCache {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Purge all cached assets across the zone
        #[arg(long, conflicts_with = "files")]
        everything: bool,
        /// Specific URL files to purge (repeatable)
        #[arg(long = "file", value_name = "URL")]
        files: Option<Vec<String>>,
    },
}

// ---------------------------------------------------------------------------------------------
// records

#[derive(Subcommand)]
pub enum Records {
    /// List the DNS records of a zone
    List {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Filter by record type, e.g. A, AAAA, CNAME, MX, TXT
        #[arg(long = "type", value_name = "TYPE")]
        record_type: Option<String>,
        /// Filter by full record name
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Records per page (default 100, max 5000)
        #[arg(long, value_name = "N", default_value = "100")]
        per_page: u32,
    },
    /// Export the zone as a BIND file
    Export {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Write to this file instead of stdout
        #[arg(long, value_name = "FILE")]
        out: Option<String>,
    },
    /// Import a BIND file into the zone
    Import {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Path to a BIND zone file
        #[arg(long, value_name = "FILE")]
        file: String,
        /// Proxy every imported record (orange cloud). Off by default
        #[arg(long)]
        proxied: bool,
    },
    /// Add a DNS record
    Add {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Record type, e.g. A, AAAA, CNAME, MX, TXT
        #[arg(long = "type", value_name = "TYPE")]
        record_type: String,
        /// Record name, e.g. connectors or connectors.example.com
        #[arg(long, value_name = "NAME")]
        name: String,
        /// Record value, e.g. an IP or target hostname
        #[arg(long, value_name = "VALUE")]
        content: String,
        /// TTL in seconds; 1 means automatic (default)
        #[arg(long, value_name = "SECONDS", default_value = "1")]
        ttl: u32,
        /// Priority, for MX and SRV records
        #[arg(long, value_name = "N")]
        priority: Option<u32>,
        /// Proxy through Cloudflare (orange cloud). A/AAAA/CNAME only
        #[arg(long)]
        proxied: bool,
    },
    /// Update a DNS record
    Update {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Record ID; alternatively identify it with --name
        #[arg(long, value_name = "ID")]
        id: Option<String>,
        /// Full record name, used to look up ID and as new name
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Record type
        #[arg(long = "type", value_name = "TYPE")]
        record_type: Option<String>,
        /// New record value
        #[arg(long, value_name = "VALUE")]
        content: Option<String>,
        /// TTL in seconds; 1 means automatic
        #[arg(long, value_name = "SECONDS")]
        ttl: Option<u32>,
        /// Proxy through Cloudflare (true or false)
        #[arg(long, value_name = "BOOL")]
        proxied: Option<bool>,
    },
    /// Delete a DNS record
    Delete {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Record ID; alternatively identify it with --name
        #[arg(long, value_name = "ID")]
        id: Option<String>,
        /// Full record name
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Record type, to disambiguate --name
        #[arg(long = "type", value_name = "TYPE")]
        record_type: Option<String>,
    },
    /// Turn the Cloudflare proxy on (orange) or off (grey) for a record
    Proxy {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Full record name, e.g. connectors.example.com
        #[arg(long, value_name = "NAME")]
        name: String,
        /// Record type, to disambiguate the name
        #[arg(long = "type", value_name = "TYPE")]
        record_type: Option<String>,
        /// Proxy through Cloudflare (orange cloud)
        #[arg(long, conflicts_with = "off")]
        on: bool,
        /// DNS only (grey cloud)
        #[arg(long, conflicts_with = "on")]
        off: bool,
    },
}

// ---------------------------------------------------------------------------------------------
// settings

#[derive(Subcommand)]
pub enum Settings {
    /// List all settings of a zone
    List(ZoneAuth),
    /// Get a single zone setting
    Get {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Setting ID, e.g. ssl, always_use_https
        #[arg(long, value_name = "NAME")]
        name: String,
    },
    /// Set a zone setting, e.g. ssl or always_use_https
    Set {
        #[command(flatten)]
        zone: ZoneAuth,
        /// Setting ID, e.g. ssl, always_use_https, min_tls_version
        #[arg(long, value_name = "NAME")]
        name: String,
        /// Setting value, e.g. full, on, off
        #[arg(long, value_name = "VALUE")]
        value: String,
    },
}

// ---------------------------------------------------------------------------------------------
// ssl

#[derive(Subcommand)]
pub enum Ssl {
    /// Show encryption mode, Universal SSL and edge certificate status
    Status(ZoneAuth),
}

// ---------------------------------------------------------------------------------------------
// pagerules

#[derive(Subcommand)]
pub enum PageRules {
    /// List the page rules of a zone
    List(ZoneAuth),
    /// Add a page rule, e.g. to force HTTPS on one hostname
    Add {
        #[command(flatten)]
        zone: ZoneAuth,
        /// URL pattern, e.g. store.example.com/*
        #[arg(long, value_name = "PATTERN")]
        url: String,
        /// Apply the always_use_https action
        #[arg(long)]
        always_use_https: bool,
        /// Rule priority; lower numbers are evaluated first (default: 1)
        #[arg(long, value_name = "N", default_value = "1")]
        priority: u32,
        /// Create the rule but leave it disabled
        #[arg(long)]
        disabled: bool,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn command_tree_is_valid() {
        use clap::CommandFactory;
        super::Cli::command().debug_assert();
    }
}
