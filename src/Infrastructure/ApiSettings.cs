using System.ComponentModel;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Infrastructure;

public class ApiSettings : CommandSettings
{
    [CommandOption("--api-token <TOKEN>")]
    [Description("Cloudflare API token (or set CLOUDFLARE_API_TOKEN env var)")]
    public string? ApiToken { get; init; }

    [CommandOption("--endpoint <URL>")]
    [Description("API base URL (or set CLOUDFLARE_API_ENDPOINT env var)")]
    public string? Endpoint { get; init; }

    public CloudflareClient CreateClient()
    {
        var token = ApiToken ?? Environment.GetEnvironmentVariable("CLOUDFLARE_API_TOKEN")
            ?? throw new InvalidOperationException(
                "API token required. Use --api-token or set CLOUDFLARE_API_TOKEN.");
        var endpoint = Endpoint ?? Environment.GetEnvironmentVariable("CLOUDFLARE_API_ENDPOINT");
        return new CloudflareClient(token, endpoint);
    }
}

/// <summary>Settings for any command that operates on a single zone.</summary>
public class ZoneSettings : ApiSettings
{
    [CommandOption("--zone <ZONE>")]
    [Description("Zone name (e.g. example.com) or zone id")]
    public required string Zone { get; init; }
}
