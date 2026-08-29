using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Records;

public sealed class DeleteRecordCommand : AsyncCommand<DeleteRecordCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--id <ID>")]
        [Description("Record id; alternatively identify it with --name")]
        public string? Id { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("Full record name")]
        public string? Name { get; init; }

        [CommandOption("--type <TYPE>")]
        [Description("Record type, to disambiguate --name")]
        public string? Type { get; init; }
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

        YamlOutput.Write(await client.DeleteAsync($"zones/{zoneId}/dns_records/{id}"));
        AnsiConsole.MarkupLine("[green]Record deleted.[/]");
        return 0;
    }
}
