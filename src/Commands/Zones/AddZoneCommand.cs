using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.Zones;

public sealed class AddZoneCommand : AsyncCommand<AddZoneCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandOption("--name <DOMAIN>")]
        [Description("The domain to add, e.g. example.com")]
        public required string Name { get; init; }

        [CommandOption("--account-id <ID>")]
        [Description("Cloudflare account id (or set CLOUDFLARE_ACCOUNT_ID env var)")]
        public string? AccountId { get; init; }

        [CommandOption("--jump-start")]
        [Description("Let Cloudflare scan for existing DNS records. Off by default: the scan is " +
                     "best-effort and silently misses records. Prefer 'records import' with a BIND file.")]
        public bool JumpStart { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var accountId = settings.AccountId ?? Environment.GetEnvironmentVariable("CLOUDFLARE_ACCOUNT_ID")
            ?? throw new InvalidOperationException(
                "Account id required. Use --account-id or set CLOUDFLARE_ACCOUNT_ID.");

        var client = settings.CreateClient();
        var result = await client.PostAsync("zones", new
        {
            name = settings.Name,
            account = new { id = accountId },
            jump_start = settings.JumpStart
        });

        YamlOutput.Write(result);
        AnsiConsole.MarkupLine($"[green]Zone {settings.Name} created. Set the name servers above at your registrar.[/]");
        return 0;
    }
}
