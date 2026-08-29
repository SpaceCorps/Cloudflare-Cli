# Cloudflare CLI

A command-line tool for the [Cloudflare](https://www.cloudflare.com) v4 API. Manage zones, DNS
records, zone settings, and page rules from the terminal.

Built for zone migrations: the BIND import/export commands make moving a zone from another DNS
provider a file transfer rather than twenty hand-typed records.

## Installation

```bash
dotnet tool install -g Cloudflare.Console
```

## Authentication

Create an API token at **My Profile → API Tokens** with these permissions, then:

| Scope | Permission | Access | Needed for |
|-------|-----------|--------|------------|
| Account | Zone | Edit | `zones add` |
| Zone | Zone | Read | zone name resolution, `zones list/get` |
| Zone | DNS | Edit | all `records` commands |
| Zone | Zone Settings | Edit | `settings set` |
| Zone | Page Rules | Edit | `pagerules` |
| Zone | SSL and Certificates | Read | `ssl status` |


```bash
export CLOUDFLARE_API_TOKEN=your-token
export CLOUDFLARE_ACCOUNT_ID=your-account-id   # only needed for 'zones add'
```

Or pass it per call with `--api-token`. The option takes precedence over the environment variable.

Anywhere a `--zone` is required you can give either the zone **name** (`example.com`) or its
32-character id; names are resolved automatically.

## Usage

```bash
cloudflare <command> [options]
```

### Commands

| Command | Description |
|---------|-------------|
| **Zones** | |
| `zones list` | List all zones in the account |
| `zones get` | Get a zone, including its status and assigned name servers |
| `zones add` | Add a domain as a new zone |
| **Records** | |
| `records list` | List the DNS records of a zone |
| `records export` | Export the zone as a BIND file |
| `records import` | Import a BIND file into the zone |
| `records add` | Add a DNS record |
| `records update` | Update a DNS record |
| `records delete` | Delete a DNS record |
| `records proxy` | Turn the proxy on (orange) or off (grey) for a record |
| **Settings** | |
| `settings list` | List all settings of a zone |
| `settings get` | Get a single zone setting |
| `settings set` | Set a zone setting, e.g. `ssl` or `always_use_https` |
| **SSL** | |
| `ssl status` | Show encryption mode, Universal SSL and edge certificate status |
| **Page rules** | |
| `pagerules list` | List the page rules of a zone |
| `pagerules add` | Add a page rule, e.g. to force HTTPS on one hostname |

## Output

Everything prints as YAML, which is easy to read and easy to parse. Status messages go to
stdout in colour; errors return a non-zero exit code.

## Examples

Migrate a zone from another provider without relying on Cloudflare's record scan:

```bash
cloudflare zones add --name example.com                       # note the name servers it returns
cloudflare records import --zone example.com --file zone.txt  # BIND file from the old provider
cloudflare records export --zone example.com --out check.txt  # diff this against zone.txt
```

`records import` leaves every record unproxied unless you pass `--proxied`, which is what a
lift-and-shift wants: identical behaviour first, changes afterwards.

Put one hostname behind the Cloudflare proxy while leaving the rest untouched:

```bash
cloudflare records proxy --zone example.com --name app.example.com --on
cloudflare records proxy --zone example.com --name app.example.com --off
```

Serve plain HTTP without redirecting to HTTPS — needed when a client cannot follow redirects:

```bash
cloudflare settings set --zone example.com --name ssl --value full
cloudflare settings set --zone example.com --name always_use_https --value off
```

`always_use_https` is zone-wide. To keep it off globally but still force HTTPS on one hostname,
use a page rule:

```bash
cloudflare pagerules add --zone example.com --url "secure.example.com/*" --always-use-https
```

Check that HTTPS is actually live before relying on it:

```bash
cloudflare ssl status --zone example.com
```

## Notes

- `zones add` does not run Cloudflare's DNS scan unless you pass `--jump-start`. The scan is
  best-effort and can silently miss records; importing a BIND file is more reliable.
- Only A, AAAA and CNAME records can be proxied. MX and TXT are always DNS-only — Cloudflare
  does not proxy SMTP.

## License

MIT
