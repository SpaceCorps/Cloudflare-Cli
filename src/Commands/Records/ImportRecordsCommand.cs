using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

/// <summary>
/// Imports a BIND zone file. Far more reliable than Cloudflare's DNS scan, which is best-effort
/// and can silently miss records - MX records in particular.
/// </summary>
public sealed class ImportRecordsCommand : AsyncCommand<ImportRecordsCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--file <FILE>")]
        [Description("Path to a BIND zone file")]
        public required string File { get; init; }

        [CommandOption("--proxied")]
        [Description("Proxy every imported record (orange cloud). Off by default, which is what " +
                     "a lift-and-shift migration wants; proxy individual records afterwards.")]
        public bool Proxied { get; init; }
    }

    public override ValidationResult Validate(CommandContext context, Settings settings) =>
        System.IO.File.Exists(settings.File)
            ? ValidationResult.Success()
            : ValidationResult.Error($"File not found: {settings.File}");

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        var content = await System.IO.File.ReadAllTextAsync(settings.File);

        var result = await client.PostFileAsync(
            $"zones/{zoneId}/dns_records/import",
            content,
            new Dictionary<string, string> { ["proxied"] = settings.Proxied ? "true" : "false" });

        YamlOutput.Write(result);
        AnsiConsole.MarkupLine("[green]Import complete. Compare 'recs_added' against the source zone.[/]");
        return 0;
    }
}
