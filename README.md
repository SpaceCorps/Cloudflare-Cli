# Cloudflare CLI

[![CI](https://github.com/SpaceCorps/Cloudflare-Cli/actions/workflows/ci.yml/badge.svg)](https://github.com/SpaceCorps/Cloudflare-Cli/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/SpaceCorps/Cloudflare-Cli?style=flat&color=38bdf8)](https://github.com/SpaceCorps/Cloudflare-Cli/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024%20Edition-orange.svg)](https://www.rust-lang.org/)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue.svg)](https://spacecorps.github.io/Cloudflare-Cli/)
[![llms.txt](https://img.shields.io/badge/llms.txt-available-purple.svg)](https://spacecorps.github.io/Cloudflare-Cli/llms.txt)

A blazing-fast, standalone native Rust command-line tool and agent interface for the **[Cloudflare](https://www.cloudflare.com) v4 REST API**.

Manage zones, DNS records, BIND file migrations, edge proxy toggles, SSL certificates, zone settings, page rules, and edge cache purges directly from your terminal or AI tool-calling workflows.

---

## Highlights

- **⚡ Sub-Millisecond Cold Starts:** Compiled in Rust Edition 2024 with LTO and zero runtime dependencies (no .NET CLR or Python).
- **🔒 Native Hardware Keystores:** Stores API tokens securely in macOS Keychain, Windows DPAPI, or Linux Secret Service (`secret-tool`). No plaintext token exposure.
- **📁 Zero-Loss BIND Migrations:** Import and export standard BIND zone files to migrate DNS providers cleanly without relying on Cloudflare's lossy automatic record scan.
- **🤖 Built for LLMs & AI Agents:** Default readable YAML output, raw JSON with `--json`, structured error envelopes on `stderr`, and built-in `<bin> agent-readme` self-documentation.
- **🌐 Edge SSL & Cache Control:** Check encryption mode, Universal SSL status, and certificate issuance in one command (`ssl status`), and purge edge caches instantly.

---

## Installation

### Precompiled Binaries

Download standalone precompiled binaries from the **[Releases](https://github.com/SpaceCorps/Cloudflare-Cli/releases)** page:

| Platform | Architecture | Binary Archive |
|---|---|---|
| **macOS** | Apple Silicon (M1/M2/M3/M4) | `cloudflare-*-aarch64-apple-darwin.tar.gz` |
| **macOS** | Intel x86_64 | `cloudflare-*-x86_64-apple-darwin.tar.gz` |
| **Linux** | x86_64 (static musl) | `cloudflare-*-x86_64-unknown-linux-musl.tar.gz` |
| **Linux** | ARM64 (static musl) | `cloudflare-*-aarch64-unknown-linux-musl.tar.gz` |
| **Windows**| x64 | `cloudflare-*-x86_64-pc-windows-msvc.zip` |

### Cargo Install

Install directly from source via `cargo`:

```bash
cargo install --git https://github.com/SpaceCorps/Cloudflare-Cli --locked
```

---

## Authentication & Accounts

### 1. Interactive Browser Flow (`login`)

Run `cloudflare login` on your developer workstation. It opens your browser to generate a token, prompts for it securely without terminal echo, verifies it against the Cloudflare API, and saves it to your OS keystore:

```bash
cloudflare login [account_name]
```

### 2. Multi-Account Management (`accounts`)

Manage multiple isolated client or staging/production profiles:

```bash
# Add named account profile
cloudflare accounts add prod --api-token <token> [--account-id <id>]

# Non-interactive via stdin (ideal for automation or clipboard paste)
pbpaste | cloudflare accounts add prod --api-token-stdin

# List configured profiles and verify connectivity
cloudflare accounts list --check

# Test stored token
cloudflare accounts test prod

# Remove profile
cloudflare accounts remove prod --yes
```

### 3. Environment Variables & CI/CD

For headless CI/CD runners or scripting, provide credentials via environment variables:

```bash
export CLOUDFLARE_API_TOKEN="your-token"
export CLOUDFLARE_ACCOUNT_ID="your-account-id"   # required for 'zones add'

cloudflare zones list
cloudflare records list --zone example.com
```

Or pass `--api-token <token>` explicitly per command.

### Required Token Permissions

Create an API token at **[Cloudflare Dashboard → My Profile → API Tokens](https://dash.cloudflare.com/profile/api-tokens)** with these permissions:

| Scope | Permission | Access | Needed for |
|---|---|---|---|
| Account | Zone | Edit | `zones add` |
| Zone | Zone | Read | zone name resolution, `zones list`, `zones get` |
| Zone | DNS | Edit | all `records` commands (`list`, `add`, `update`, `delete`, `proxy`, `import`, `export`) |
| Zone | Zone Settings | Edit | `settings list`, `settings get`, `settings set` |
| Zone | Page Rules | Edit | `pagerules list`, `pagerules add` |
| Zone | SSL and Certificates | Read | `ssl status` |
| Zone | Cache Purge | Purge | `zones purge-cache` |

> [!NOTE]
> Anywhere `--zone` is expected, you can pass either the zone **domain name** (`example.com`) or its 32-character hex ID. Domain names are resolved to IDs automatically.

---

## Command Reference

```bash
cloudflare <command> [subcommand] [options]
```

| Group | Subcommand | Description |
|---|---|---|
| **Zones** | `zones list` | List all zones in the account |
| | `zones get --zone <Z>` | Inspect zone status and assigned nameservers |
| | `zones add --name <DOMAIN>` | Add a domain as a new zone |
| | `zones purge-cache --zone <Z>` | Purge edge cache (`--everything` or `--file <URL>`) |
| **Records** | `records list --zone <Z>` | List DNS records (optional `--type`, `--name`, `--per-page`) |
| | `records export --zone <Z>` | Export zone as a standard BIND zone file (optional `--out <FILE>`) |
| | `records import --zone <Z> --file <F>` | Import a BIND zone file into Cloudflare (optional `--proxied`) |
| | `records add --zone <Z>` | Add a DNS record (A, AAAA, CNAME, MX, TXT, SRV) |
| | `records update --zone <Z>` | Update record value, TTL, or proxy status by `--id` or `--name` |
| | `records proxy --zone <Z>` | Toggle Cloudflare edge proxy on (`--on`) or off (`--off`) |
| | `records delete --zone <Z>` | Delete a DNS record by `--id` or `--name` |
| **Settings** | `settings list --zone <Z>` | List all settings of a zone |
| | `settings get --zone <Z> --name <N>` | Read a single zone setting |
| | `settings set --zone <Z> --name <N>` | Set a zone setting, e.g. `ssl` or `always_use_https` |
| **SSL** | `ssl status --zone <Z>` | Inspect encryption mode, Universal SSL & certificate packs |
| **Page Rules**| `pagerules list --zone <Z>` | List configured page rules |
| | `pagerules add --zone <Z>` | Add a page rule, e.g. enforce HTTPS on specific subdomains |
| **Accounts** | `login [name]` | Browser token login and OS keystore storage |
| | `accounts list` | List configured account profiles (optional `--check`) |
| | `accounts test <name>` | Verify that a stored account's token is valid |
| | `accounts remove <name>` | Remove an account profile and delete its stored token |
| **Manual** | `agent-readme` | Print agent operating manual (Markdown or `--json`) |

---

## Migration & Usage Examples

### 1. Zero-Loss DNS Migration via BIND Zone File

Avoid Cloudflare's best-effort DNS scan, which frequently misses MX or SRV records:

```bash
# 1. Create the zone on Cloudflare (note the assigned nameservers)
cloudflare zones add --name example.com

# 2. Import the complete BIND zone file from your previous registrar/provider
cloudflare records import --zone example.com --file old_provider_zone.txt

# 3. Export back and diff against source to verify 100% record fidelity
cloudflare records export --zone example.com --out verify_zone.txt
diff old_provider_zone.txt verify_zone.txt
```

### 2. Toggle Cloudflare Edge Proxy (Orange Cloud)

Route specific hosts through the Cloudflare edge while leaving apex or mail records untouched:

```bash
cloudflare records proxy --zone example.com --name app.example.com --on
cloudflare records proxy --zone example.com --name api.example.com --off
```

### 3. Front a Redirect-Intolerant Origin (Plain HTTP)

```bash
cloudflare settings set --zone example.com --name ssl --value full
cloudflare settings set --zone example.com --name always_use_https --value off
```

To enforce HTTPS only on specific paths:
```bash
cloudflare pagerules add --zone example.com --url "secure.example.com/*" --always-use-https
```

### 4. Verify Live SSL Issuance

Check that edge certificates are active before updating registrar nameservers:

```bash
cloudflare ssl status --zone example.com
```

### 5. Purge Edge Cache

```bash
# Purge everything
cloudflare zones purge-cache --zone example.com --everything

# Purge specific files
cloudflare zones purge-cache --zone example.com --file https://example.com/style.css --file https://example.com/app.js
```

---

## Agentic Interface & Error Protocol

All commands support `--json` for structured output.

Errors are output to `stderr` with a stable error envelope and corresponding exit code:

```json
{
  "code": "auth_required",
  "error": "Cloudflare API error: [10000] Authentication error",
  "detail": "HTTP 403: [10000] Authentication error",
  "remediation": "Check your API token and permissions: https://dash.cloudflare.com/profile/api-tokens"
}
```

| Exit Code | Error Code | Meaning |
|---|---|---|
| `0` | `ok` | Command completed successfully |
| `1` | `error` | Unclassified error |
| `2` | `network` | Network timeout or transport failure (retry once) |
| `3` | `auth_required` | Token rejected, missing permissions, or expired |
| `4` | `not_found` | Zone or record not found |
| `5` | `rate_limited` | Cloudflare API rate limit reached (back off) |
| `6` | `invalid_input` | Missing required argument or invalid parameter |
| `7` | `no_account` | No account profile or token provided |

---

## Lineage & Acknowledgements

This tool is a native Rust port of the original `.NET` global tool `Cloudflare.Console` created by **[Niels Bosma](https://github.com/nielsbosma)**. The Rust port is engineered and maintained by the **SpaceCorps** open-source collective.

## License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.
