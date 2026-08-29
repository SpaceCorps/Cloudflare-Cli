using System.ComponentModel;
using Cloudflare.Console.Infrastructure;
using Spectre.Console;
using Spectre.Console.Cli;

namespace Cloudflare.Console.Commands.PageRules;

/// <summary>
/// "Always Use HTTPS" is a zone-wide setting, so when one hostname must keep serving plain HTTP
/// it has to stay off globally. A page rule then enforces HTTPS on the hostnames that can take it.
/// The free plan includes three page rules.
/// </summary>
public sealed class AddPageRuleCommand : AsyncCommand<AddPageRuleCommand.Settings>
{
    public sealed class Settings : ZoneSettings
    {
        [CommandOption("--url <PATTERN>")]
        [Description("URL pattern, e.g. store.example.com/*")]
        public required string Url { get; init; }

        [CommandOption("--always-use-https")]
        [Description("Apply the always_use_https action")]
        public bool AlwaysUseHttps { get; init; }

        [CommandOption("--priority <N>")]
        [Description("Rule priority; lower numbers are evaluated first")]
        public int Priority { get; init; } = 1;

        [CommandOption("--disabled")]
        [Description("Create the rule but leave it disabled")]
        public bool Disabled { get; init; }
    }

    public override ValidationResult Validate(CommandContext context, Settings settings) =>
        settings.AlwaysUseHttps
            ? ValidationResult.Success()
            : ValidationResult.Error("No action specified. Pass --always-use-https.");

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var zoneId = await client.ResolveZoneIdAsync(settings.Zone);

        var body = new Dictionary<string, object?>
        {
            ["targets"] = new object[]
            {
                new
                {
                    target = "url",
                    constraint = new { @operator = "matches", value = settings.Url }
                }
            },
            ["actions"] = new object[] { new { id = "always_use_https" } },
            ["priority"] = settings.Priority,
            ["status"] = settings.Disabled ? "disabled" : "active"
        };

        YamlOutput.Write(await client.PostAsync($"zones/{zoneId}/pagerules", body));
        AnsiConsole.MarkupLine($"[green]Page rule created for {settings.Url}[/]");
        return 0;
    }
}
