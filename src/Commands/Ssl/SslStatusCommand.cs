using Cloudflare.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Ssl;

/// <summary>
/// Answers "is HTTPS actually working on this zone" in one call, by combining the three things
/// that have to line up: the encryption mode, whether Universal SSL is on, and whether the edge
/// certificates have finished issuing.
///
/// Needs the SSL and Certificates : Read permission in addition to Zone Settings.
/// </summary>
public sealed class SslStatusCommand : AsyncCommand<ZoneSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, ZoneSettings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);

        var mode = await SafeGetAsync(client, $"zones/{zoneId}/settings/ssl");
        var alwaysHttps = await SafeGetAsync(client, $"zones/{zoneId}/settings/always_use_https");
        var universal = await SafeGetAsync(client, $"zones/{zoneId}/ssl/universal/settings");
        var packs = await SafeGetAsync(client, $"zones/{zoneId}/ssl/certificate_packs?status=all");

        YamlOutput.Write(new Dictionary<string, object?>
        {
            ["encryption_mode"] = Value(mode),
            ["always_use_https"] = Value(alwaysHttps),
            ["universal_ssl_enabled"] = universal is Dictionary<string, object?> u
                && u.TryGetValue("enabled", out var enabled) ? enabled : universal,
            ["certificate_packs"] = Summarize(packs)
        });

        return 0;
    }

    /// <summary>A missing permission should not sink the whole report - show what is readable.</summary>
    private static async Task<object?> SafeGetAsync(CloudflareClient client, string path)
    {
        try
        {
            return await client.GetAsync(path);
        }
        catch (CloudflareApiException ex)
        {
            return $"unavailable: {ex.Message}";
        }
    }

    private static object? Value(object? setting) =>
        setting is Dictionary<string, object?> d && d.TryGetValue("value", out var v) ? v : setting;

    private static object? Summarize(object? packs)
    {
        if (packs is not List<object?> list) return packs;

        return list.OfType<Dictionary<string, object?>>().Select(p => new Dictionary<string, object?>
        {
            ["type"] = p.GetValueOrDefault("type"),
            ["status"] = p.GetValueOrDefault("status"),
            ["hosts"] = p.GetValueOrDefault("hosts"),
            ["certificate_authority"] = p.GetValueOrDefault("certificate_authority")
        }).ToList();
    }
}
