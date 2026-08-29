using Cloudflare.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Zones;

public sealed class ListZonesCommand : AsyncCommand<ApiSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, ApiSettings settings)
    {
        var client = settings.CreateClient();
        YamlOutput.Write(await client.GetAsync("zones"));
        return 0;
    }
}
