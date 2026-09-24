# AGENTS.md

Developer and agent engineering notes for `cloudflare`.

`cloudflare` is a high-performance native Rust CLI over the Cloudflare v4 REST API, built to be driven by humans and autonomous LLM agents. It replaces Niels Bosma's original `Cloudflare.Console` .NET prototype tool, providing identical functional commands, YAML-first output, `--json` mode, structured error envelopes, and native OS keystore credential storage.

For the manual the *agent* reads at runtime, run `cloudflare agent-readme` — that text lives in `src/readme.rs`. This file is for developers maintaining and extending the source code.

---

## Commands & Verification

```bash
cargo build --release              # target/release/cloudflare
cargo test --locked                # unit tests + tests/cli.rs against mock API
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo install --path . --locked    # install binary on PATH
```

### Testing with Isolated Credentials

Use throwaway configuration directories when testing so that real credentials are never touched:

```bash
export CLOUDFLARE_CONFIG_DIR=$(mktemp -d)
export CLOUDFLARE_SECRET_STORE=plaintext
export CLOUDFLARE_ALLOW_PLAINTEXT_STORE=1
```

| Environment Variable | Effect |
|---|---|
| `CLOUDFLARE_CONFIG_DIR` | Overrides configuration and secrets storage directory |
| `CLOUDFLARE_SECRET_STORE` | Forces a backend: `dpapi`, `keychain`, `libsecret`, `plaintext` |
| `CLOUDFLARE_ALLOW_PLAINTEXT_STORE=1` | Permits fallback to 0600 JSON file when no OS keystore is available |
| `CLOUDFLARE_API_URL` | Overrides the API base URL (used by `tests/cli.rs` mock server) |
| `CLOUDFLARE_API_TOKEN` | Direct token authentication fallback for non-interactive runners |
| `CLOUDFLARE_ACCOUNT_ID` | Account ID fallback for `zones add` |

---

## Architecture & Code Layout

```
src/
  main.rs          arg parsing, --json pre-scan, clap error formatting to invalid_input envelopes
  cli.rs           clap derive hierarchy, command arguments, and documentation
  client.rs        blocking HTTP (ureq + rustls), status -> ErrorCode mapping, Cloudflare envelope unwrapping
  error.rs         ErrorCode enum (= exit code) and structured Error {message, detail, remediation}
  output.rs        YAML by default (serde_norway), JSON with --json (serde_json), obj! macro
  account.rs       multi-account resolution, AuthArgs -> Resolved context
  config.rs        config.yaml metadata, atomic writes, 0600 permissions, cross-process lock
  secrets.rs       keystore integration (macOS Keychain, Linux secret-tool, Windows DPAPI, plaintext fallback)
  readme.rs        agent-readme manual text and machine-readable data
  commands/
    mod.rs         command dispatcher, resolve helpers, done() envelope helper
    accounts.rs    accounts add|list|test|remove
    login.rs       interactive browser token flow and keystore storage
    zones.rs       zones list|get|add|purge-cache
    records.rs     records list|export|import|add|update|delete|proxy
    settings.rs    settings list|get|set
    ssl.rs         ssl status (consolidated encryption, Universal SSL, and cert packs)
    pagerules.rs   pagerules list|add
tests/
  cli.rs           in-process TCP mock HTTP server covering all subcommands and error mappings
docs/
  index.html       responsive dark glassmorphic landing page with Schema.org JSON-LD
  index.md         clean markdown mirror of the landing page
  llms.txt         standard agent discovery and summary manual
  llms-full.txt    exhaustive agent reference manual
  auth.md          authentication and permission scope documentation
  pricing.md       open-source and Cloudflare tier reference
  about.html       project lineage and engineering mission
  contact.html     technical support and issue reporting
  privacy.html     zero-telemetry and keystore security disclosure
  404.html         not found landing page
  robots.txt       crawler permissions for search engines and AI user-agents
  sitemap.xml      search engine indexing sitemap
  .well-known/
    agent-card.json   A2A agent discovery card
    agent-skills/     AgentSkills discovery manifest (schema 0.2.0)
```

---

## Core Invariants

1. **Blocking HTTP over Tokio**: A CLI makes only a handful of HTTP calls per invocation. An async runtime would increase binary size and cold-start latency. `ureq` with `rustls` provides instant 1–3 ms execution.
2. **Deterministic Multi-Account Safety**: API operations use named accounts stored in OS keystores, or explicit `--api-token` / `CLOUDFLARE_API_TOKEN` in CI. No silent deletion across production domains.
3. **No Secrets in Config**: Tokens are encrypted in the OS keystore. `config.yaml` stores only account alias names, account IDs, and timestamps.
4. **Stable Exit Codes**: Machine-readable `code:` field in stderr matches numeric exit codes:
   - `0`: ok
   - `1`: error
   - `2`: network
   - `3`: auth_required
   - `4`: not_found
   - `5`: rate_limited
   - `6`: invalid_input
   - `7`: no_account
5. **Lossless BIND Migrations**: `records import` does not proxy records by default, allowing lift-and-shift DNS migrations without unintended proxy side-effects.

---

## Release Pipeline

GitHub Actions (`.github/workflows/ci.yml`) runs tests, format check, and clippy on macOS, Linux, and Windows for every pull request and push to `main`. Creating a release tag `v*` packages and uploads 5 cross-platform binaries:
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-unknown-linux-musl` (Linux x86_64 static)
- `aarch64-unknown-linux-musl` (Linux ARM64 static)
- `x86_64-pc-windows-msvc` (Windows x64)
