using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

public sealed class UpdateRecordCommand : AsyncCommand<UpdateRecordCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--id <ID>")]
        [Description("Record id; alternatively identify it with --name")]
        public string? Id { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("Full record name, used to look up the id and as the new name")]
        public string? Name { get; init; }

        [CommandOption("--type <TYPE>")]
        [Description("Record type")]
        public string? Type { get; init; }

        [CommandOption("--content <VALUE>")]
        [Description("New record value")]
        public string? Content { get; init; }

        [CommandOption("--ttl <SECONDS>")]
        [Description("TTL in seconds; 1 means automatic")]
        public int? Ttl { get; init; }

        [CommandOption("--proxied <BOOL>")]
        [Description("true or false - proxy through Cloudflare")]
        public bool? Proxied { get; init; }
    }

    public override ValidationResult Validate(CommandContext context, Settings settings) =>
        string.IsNullOrWhiteSpace(settings.Id) && string.IsNullOrWhiteSpace(settings.Name)
            ? ValidationResult.Error("Either --id or --name is required.")
            : ValidationResult.Success();

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);
        var id = settings.Id ?? await client.ResolveRecordIdAsync(zoneId, settings.Name!, settings.Type);

        var body = new Dictionary<string, object?>();
        if (settings.Type is not null) body["type"] = settings.Type.ToUpperInvariant();
        if (settings.Name is not null) body["name"] = settings.Name;
        if (settings.Content is not null) body["content"] = settings.Content;
        if (settings.Ttl is not null) body["ttl"] = settings.Ttl;
        if (settings.Proxied is not null) body["proxied"] = settings.Proxied;

        YamlOutput.Write(await client.PatchAsync($"zones/{zoneId}/dns_records/{id}", body));
        AnsiConsole.MarkupLine("[green]Record updated.[/]");
        return 0;
    }
}
