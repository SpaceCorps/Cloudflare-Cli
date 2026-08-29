using Cloudflare.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.PageRules;

public sealed class ListPageRulesCommand : AsyncCommand<ZoneSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, ZoneSettings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        YamlOutput.Write(await client.GetAsync($"zones/{zoneId}/pagerules"));
        return 0;
    }
}
