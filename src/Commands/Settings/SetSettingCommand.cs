using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Settings;

/// <summary>
/// Sets a zone setting. The two that matter for fronting a redirect-intolerant origin are:
///   ssl               off | flexible | full | strict
///   always_use_https  on | off        - must be "off" to serve plain HTTP without a 301
/// </summary>
public sealed class SetSettingCommand : AsyncCommand<SetSettingCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--name <NAME>")]
        [Description("Setting id, e.g. ssl, always_use_https, min_tls_version")]
        public required string Name { get; init; }

        [CommandOption("--value <VALUE>")]
        [Description("Setting value, e.g. full, on, off")]
        public required string Value { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);

        var result = await client.PatchAsync(
            $"zones/{zoneId}/settings/{settings.Name}",
            new Dictionary<string, object?> { ["value"] = settings.Value });

        YamlOutput.Write(result);
        AnsiConsole.MarkupLine($"[green]{settings.Name} = {settings.Value}[/]");
        return 0;
    }
}
