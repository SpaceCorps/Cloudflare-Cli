using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Settings;

public sealed class GetSettingCommand : AsyncCommand<GetSettingCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--name <NAME>")]
        [Description("Setting id, e.g. ssl, always_use_https")]
        public required string Name { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        YamlOutput.Write(await client.GetAsync($"zones/{zoneId}/settings/{settings.Name}"));
        return 0;
    }
}
