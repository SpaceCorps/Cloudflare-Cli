using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

/// <summary>
/// The grey/orange cloud toggle. Orange routes traffic through the Cloudflare edge, which is what
/// allows a hostname to answer on plain HTTP without redirecting to HTTPS.
/// </summary>
public sealed class ProxyRecordCommand : AsyncCommand<ProxyRecordCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--name <NAME>")]
        [Description("Full record name, e.g. connectors.example.com")]
        public required string Name { get; init; }

        [CommandOption("--type <TYPE>")]
        [Description("Record type, to disambiguate the name")]
        public string? Type { get; init; }

        [CommandOption("--on")]
        [Description("Proxy through Cloudflare (orange cloud)")]
        public bool On { get; init; }

        [CommandOption("--off")]
        [Description("DNS only (grey cloud)")]
        public bool Off { get; init; }
    }

    public override ValidationResult Validate(CommandContext context, Settings settings) =>
        settings.On == settings.Off
            ? ValidationResult.Error("Specify exactly one of --on or --off.")
            : ValidationResult.Success();

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        var id = await client.ResolveRecordIdAsync(zoneId, settings.Name, settings.Type);

        var result = await client.PatchAsync($"zones/{zoneId}/dns_records/{id}",
            new Dictionary<string, object?> { ["proxied"] = settings.On });

        YamlOutput.Write(result);
        AnsiConsole.MarkupLine(settings.On
            ? $"[green]{settings.Name} is now proxied (orange cloud).[/]"
            : $"[green]{settings.Name} is now DNS only (grey cloud).[/]");
        return 0;
    }
}
