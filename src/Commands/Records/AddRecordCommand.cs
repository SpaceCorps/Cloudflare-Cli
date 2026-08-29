using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

public sealed class AddRecordCommand : AsyncCommand<AddRecordCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--type <TYPE>")]
        [Description("Record type, e.g. A, AAAA, CNAME, MX, TXT")]
        public required string Type { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("Record name, e.g. connectors or connectors.example.com")]
        public required string Name { get; init; }

        [CommandOption("--content <VALUE>")]
        [Description("Record value, e.g. an IP or target hostname")]
        public required string Content { get; init; }

        [CommandOption("--ttl <SECONDS>")]
        [Description("TTL in seconds; 1 means automatic (default)")]
        public int Ttl { get; init; } = 1;

        [CommandOption("--priority <N>")]
        [Description("Priority, for MX and SRV records")]
        public int? Priority { get; init; }

        [CommandOption("--proxied")]
        [Description("Proxy through Cloudflare (orange cloud). A/AAAA/CNAME only.")]
        public bool Proxied { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);

        var body = new Dictionary<string, object?>
        {
            ["type"] = settings.Type.ToUpperInvariant(),
            ["name"] = settings.Name,
            ["content"] = settings.Content,
            ["ttl"] = settings.Ttl,
            ["proxied"] = settings.Proxied
        };
        if (settings.Priority is not null) body["priority"] = settings.Priority;

        YamlOutput.Write(await client.PostAsync($"zones/{zoneId}/dns_records", body));
        AnsiConsole.MarkupLine($"[green]{settings.Type.ToUpperInvariant()} record {settings.Name} created.[/]");
        return 0;
    }
}
