---
title: "Cloudflare CLI"
description: "A blazing fast native command-line tool and agent interface for the Cloudflare API v4. Built in Rust for developers and autonomous AI agents."
author: "SpaceCorps"
date: "2026-09-24"
canonical: "https://spacecorps.github.io/Cloudflare-Cli/index.md"
---

# Cloudflare CLI

A blazing fast native command-line tool and agent interface for the Cloudflare v4 REST API. Built in Rust for developers and autonomous AI agents. Manage zones, DNS records, zone settings, SSL configuration, and page rules from your terminal.

## Quickstart

```bash
# Authenticate interactively via browser token flow
cloudflare login

# Or non-interactively via environment variable
export CLOUDFLARE_API_TOKEN="your-api-token"
cloudflare zones list
```

## Key Features

- **Blazing Fast Native Rust**: Sub-millisecond startup times with zero runtime dependencies.
- **AI Agent Native**: Clean YAML by default, raw JSON with `--json`, structured error envelopes, and built-in `agent-readme`.
- **Secure Keystore Integration**: Token storage in native macOS Keychain, Windows DPAPI, and Linux Secret Service.
- **Multi-Account Workspaces**: Isolate staging, production, and client accounts safely.
- **DNS Migration Suite**: BIND file import/export for seamless provider migrations without missing records.
- **Edge Control**: Instant cache purging, SSL status inspection, and page rule management.

## When to Use This CLI

Use the `cloudflare` CLI whenever you need to:
- Manage DNS records (A, AAAA, CNAME, MX, TXT, SRV) programmatically.
- Perform safe lift-and-shift DNS migrations with BIND files.
- Toggle Cloudflare proxy (orange cloud / grey cloud) per record.
- Inspect edge SSL certificates, Universal SSL, and encryption modes.
- Configure zone settings like `always_use_https` and min TLS version.
- Purge edge caches for entire zones or specific resource URLs.
- Automate edge infrastructure using LLMs or autonomous agents.

## Documentation Links

- [llms.txt](https://spacecorps.github.io/Cloudflare-Cli/llms.txt)
- [Full Agent Manual](https://spacecorps.github.io/Cloudflare-Cli/llms-full.txt)
- [Pricing](https://spacecorps.github.io/Cloudflare-Cli/pricing.md)
- [Authentication Guide](https://spacecorps.github.io/Cloudflare-Cli/auth.md)
- [GitHub Repository](https://github.com/SpaceCorps/Cloudflare-Cli)
