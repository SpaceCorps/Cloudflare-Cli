using Cloudflare.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Zones;

/// <summary>
/// Shows the zone, including "status" (pending until the nameservers are changed at the
/// registrar) and "name_servers" (the two Cloudflare nameservers to set there).
/// </summary>
public sealed class GetZoneCommand : AsyncCommand<ZoneSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, ZoneSettings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        YamlOutput.Write(await client.GetAsync($"zones/{zoneId}"));
        return 0;
    }
}
