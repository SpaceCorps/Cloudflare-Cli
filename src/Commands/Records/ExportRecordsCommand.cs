using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

/// <summary>Exports the zone as a BIND file, for diffing against the source provider.</summary>
public sealed class ExportRecordsCommand : AsyncCommand<ExportRecordsCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--out <FILE>")]
        [Description("Write to this file instead of stdout")]
        public string? Out { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        var bind = await client.GetRawAsync($"zones/{zoneId}/dns_records/export");

        if (string.IsNullOrWhiteSpace(settings.Out))
        {
            System.Console.WriteLine(bind);
        }
        else
        {
            await File.WriteAllTextAsync(settings.Out, bind);
            AnsiConsole.MarkupLine($"[green]Exported to {settings.Out}[/]");
        }

        return 0;
    }
}
