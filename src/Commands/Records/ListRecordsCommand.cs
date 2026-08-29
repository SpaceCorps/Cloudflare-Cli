using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

public sealed class ListRecordsCommand : AsyncCommand<ListRecordsCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--type <TYPE>")]
        [Description("Filter by record type, e.g. MX, TXT, CNAME")]
        public string? Type { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("Filter by full record name")]
        public string? Name { get; init; }

        [CommandOption("--per-page <N>")]
        [Description("Records per page (default 100, max 5000)")]
        public int PerPage { get; init; } = 100;
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);

        var query = $"zones/{zoneId}/dns_records?per_page={settings.PerPage}";
        if (!string.IsNullOrWhiteSpace(settings.Type)) query += $"&type={Uri.EscapeDataString(settings.Type)}";
        if (!string.IsNullOrWhiteSpace(settings.Name)) query += $"&name={Uri.EscapeDataString(settings.Name)}";

        YamlOutput.Write(await client.GetAsync(query));
        return 0;
    }
}
