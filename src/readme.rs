//! The manual an agent reads before its first call. Markdown by default so it can be pasted
//! into a system prompt or a CLAUDE.md; `--json` gives the same rules as data.

use crate::{obj, output};

pub fn print() {
    if output::json() {
        output::write(&obj! {
            "tool" => "cloudflare",
            "apiVersion" => API_VERSION,
            "rules" => RULES,
            "exitCodes" => obj! {
                "0" => "ok",
                "1" => "error - unclassified, report and stop",
                "2" => "network - retry once, then stop",
                "3" => "auth_required - stop, surface the remediation to a human",
                "4" => "not_found - do not retry",
                "5" => "rate_limited - back off before retrying",
                "6" => "invalid_input - fix the call",
                "7" => "no_account - run cloudflare accounts list or login",
            },
        });
        return;
    }
    println!("{README}");
}

pub const API_VERSION: &str = "v4";

const RULES: &[&str] = &[
    "Always pass --account or configure CLOUDFLARE_API_TOKEN. There is no implicit default account.",
    "Run 'cloudflare accounts list' first if you do not know which accounts exist; ask the human which to use.",
    "On code auth_required, stop and surface the remediation string. Do not retry.",
    "Anywhere --zone is required, pass either the domain name (e.g. example.com) or the 32-character hex ID.",
    "Only A, AAAA, and CNAME records can be proxied (orange cloud). MX and TXT records must be DNS-only (grey cloud).",
    "Use 'records import' with a BIND file for migration instead of manual records.",
    "Deletes are irreversible and take no confirmation. Verify the resource before deleting it.",
    "Use --json when you are going to parse the output in scripts or tool loops.",
];

const README: &str = r#"# cloudflare - agent operating manual

A native CLI over the Cloudflare v4 API: manage zones, DNS records, zone settings, SSL configuration,
page rules, and cache purging. Results are YAML on stdout by default, errors are YAML on stderr,
and `--json` switches both to JSON. Status and info messages go to stderr so stdout is always clean and parseable.

## Authentication & Multi-Account Safety

`--account` (short `-a`) selects a named account profile stored in the native OS keystore
(DPAPI on Windows, Keychain on macOS, libsecret on Linux).

    cloudflare login [<name>]                  # interactive login, opens browser to create token
    cloudflare accounts add <name> --api-token <token> [--account-id <id>]
    printf %s "$TOKEN" | cloudflare accounts add <name> --api-token-stdin
    cloudflare accounts list [--check]
    cloudflare accounts test <name>
    cloudflare accounts remove <name> --yes

Alternatively, for CI/CD or non-interactive environments, pass `--api-token <token>` or set
the `CLOUDFLARE_API_TOKEN` environment variable. For zone creation (`zones add`), also set
`CLOUDFLARE_ACCOUNT_ID` or pass `--account-id`.

## Zones

    cloudflare zones list [-a <account>]
    cloudflare zones get --zone example.com [-a <account>]
    cloudflare zones add --name example.com [--account-id <id>] [--jump-start] [-a <account>]
    cloudflare zones purge-cache --zone example.com --everything [-a <account>]
    cloudflare zones purge-cache --zone example.com --files https://example.com/main.css ... [-a <account>]

Zone parameters accept either the domain name (e.g. `example.com`) or the 32-character hex ID.
Names are resolved to IDs automatically.

## DNS Records

    cloudflare records list --zone example.com [--type <TYPE>] [--name <NAME>]
    cloudflare records export --zone example.com [--out zone.txt]
    cloudflare records import --zone example.com --file zone.txt [--proxied]
    cloudflare records add --zone example.com --type A --name web --content 192.0.2.1 [--ttl 1] [--proxied]
    cloudflare records update --zone example.com (--id <ID> | --name <NAME>) [--content <IP>] [--proxied <BOOL>]
    cloudflare records delete --zone example.com (--id <ID> | --name <NAME>) [--type <TYPE>]
    cloudflare records proxy --zone example.com --name web (--on | --off)

`records import` imports a standard BIND zone file. By default records are imported as DNS-only (grey cloud),
which is the safest migration pattern. Individual records can then be toggled with `records proxy`.

## Zone Settings & SSL

    cloudflare settings list --zone example.com
    cloudflare settings get --zone example.com --name ssl
    cloudflare settings set --zone example.com --name ssl --value full
    cloudflare settings set --zone example.com --name always_use_https --value off
    cloudflare ssl status --zone example.com

`ssl status` checks encryption mode, universal SSL status, and edge certificate packs in a single consolidated report.

## Page Rules

    cloudflare pagerules list --zone example.com
    cloudflare pagerules add --zone example.com --url "store.example.com/*" --always-use-https [--priority 1]

## Errors & Exit Codes

Failures output a structured error envelope on stderr with a machine-readable `code` and matching exit status:

    0  ok
    1  error          unclassified - report it and stop
    2  network        retry once, then stop
    3  auth_required  stop; give the human the remediation string verbatim
    4  not_found      the zone or record does not exist; do not retry
    5  rate_limited   back off before trying again
    6  invalid_input  fix the call
    7  no_account     run 'cloudflare accounts list' or 'cloudflare login'
"#;
