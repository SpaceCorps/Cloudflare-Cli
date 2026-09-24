---
title: "Authentication Guide"
description: "Authentication methods, API token permissions, and credential storage for Cloudflare CLI."
author: "SpaceCorps"
date: "2026-09-24"
---

# Authentication Guide for Cloudflare CLI

This guide outlines authentication methods, required token permissions, and credential storage for developers and AI agents using the Cloudflare CLI.

## Overview

The Cloudflare CLI communicates directly with the Cloudflare v4 REST API. Authentication uses scoped API tokens issued through your Cloudflare profile dashboard. Tokens can be stored in the host operating system's native keystore or supplied directly via environment variables and CLI arguments.

## Prerequisites

- A Cloudflare account ([cloudflare.com](https://dash.cloudflare.com/))
- A scoped API token generated from **My Profile → API Tokens** (`https://dash.cloudflare.com/profile/api-tokens`)
- Cloudflare CLI installed on your system

## Required API Token Permissions

Depending on the commands you plan to execute, create an API token with the following permissions:

| Scope | Permission | Access | Commands |
|---|---|---|---|
| Account | Zone | Edit | `zones add` |
| Zone | Zone | Read | zone name resolution, `zones list`, `zones get` |
| Zone | DNS | Edit | all `records` commands (`list`, `add`, `update`, `delete`, `proxy`, `import`, `export`) |
| Zone | Zone Settings | Edit | `settings list`, `settings get`, `settings set` |
| Zone | Page Rules | Edit | `pagerules list`, `pagerules add` |
| Zone | SSL and Certificates | Read | `ssl status` |
| Zone | Cache Purge | Purge | `zones purge-cache` |

## Authentication Workflows

### 1. Interactive Login (`cloudflare login`)

Recommended for developer workstations:
```bash
cloudflare login [account_name]
```
1. The CLI opens your system browser to `https://dash.cloudflare.com/profile/api-tokens`.
2. Generate or copy your API token.
3. Paste the token into the CLI prompt (input is masked).
4. The CLI verifies the token with a live probe against `GET user/tokens/verify`.
5. Upon confirmation, the token is saved to the native OS keystore under the profile name (defaults to `default`).

### 2. Multi-Account Management (`cloudflare accounts`)

Configure isolated account profiles for multiple organizations or client environments:
```bash
# Add named account profile
cloudflare accounts add production --api-token <token> [--account-id <id>]

# Non-interactive via stdin (ideal for automation or clipboard paste)
pbpaste | cloudflare accounts add staging --api-token-stdin

# List configured profiles
cloudflare accounts list [--check]

# Test stored token connectivity
cloudflare accounts test production

# Remove profile
cloudflare accounts remove staging --yes
```

### 3. Non-Interactive / CI/CD Environments

Pass the token via environment variable or CLI argument:
```bash
export CLOUDFLARE_API_TOKEN="your-api-token"
export CLOUDFLARE_ACCOUNT_ID="your-account-id"   # needed for 'zones add'

cloudflare zones list
cloudflare records list --zone example.com
```

Or pass `--api-token` directly:
```bash
cloudflare records list --zone example.com --api-token "$CLOUDFLARE_API_TOKEN"
```

## Security Best Practices

1. **Least Privilege**: Grant only the specific zone and resource permissions required.
2. **Native Keystores**: Tokens are encrypted using macOS Keychain, Windows DPAPI, or Linux Secret Service.
3. **No Plaintext by Default**: Keystores protect secrets from disk scanning unless explicitly opted into via `CLOUDFLARE_ALLOW_PLAINTEXT_STORE=1`.
4. **Machine Verification**: Pass `--json` when scripting to ensure structured output and standard error codes.
