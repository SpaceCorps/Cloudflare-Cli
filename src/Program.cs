using Cloudflare.Console.Commands.PageRules;
using Cloudflare.Console.Commands.Records;
using Cloudflare.Console.Commands.Settings;
using Cloudflare.Console.Commands.Zones;
using Spectre.Console.Cli;

var app = new CommandApp();

app.Configure(config =>
{
    config.SetApplicationName("cloudflare");

    config.AddBranch("zones", zones =>
    {
        zones.SetDescription("Manage zones");
        zones.AddCommand<ListZonesCommand>("list")
            .WithDescription("List all zones in the account");
        zones.AddCommand<GetZoneCommand>("get")
            .WithDescription("Get a zone, including its status and assigned name servers");
        zones.AddCommand<AddZoneCommand>("add")
            .WithDescription("Add a domain as a new zone");
    });

    config.AddBranch("records", records =>
    {
        records.SetDescription("Manage DNS records");
        records.AddCommand<ListRecordsCommand>("list")
            .WithDescription("List the DNS records of a zone");
        records.AddCommand<ExportRecordsCommand>("export")
            .WithDescription("Export the zone as a BIND file");
        records.AddCommand<ImportRecordsCommand>("import")
            .WithDescription("Import a BIND file into the zone");
        records.AddCommand<AddRecordCommand>("add")
            .WithDescription("Add a DNS record");
        records.AddCommand<UpdateRecordCommand>("update")
            .WithDescription("Update a DNS record");
        records.AddCommand<DeleteRecordCommand>("delete")
            .WithDescription("Delete a DNS record");
        records.AddCommand<ProxyRecordCommand>("proxy")
            .WithDescription("Turn the Cloudflare proxy on (orange) or off (grey) for a record");
    });

    config.AddBranch("settings", settings =>
    {
        settings.SetDescription("Read and write zone settings");
        settings.AddCommand<ListSettingsCommand>("list")
            .WithDescription("List all settings of a zone");
        settings.AddCommand<GetSettingCommand>("get")
            .WithDescription("Get a single zone setting");
        settings.AddCommand<SetSettingCommand>("set")
            .WithDescription("Set a zone setting, e.g. ssl or always_use_https");
    });

    config.AddBranch("pagerules", pagerules =>
    {
        pagerules.SetDescription("Manage page rules");
        pagerules.AddCommand<ListPageRulesCommand>("list")
            .WithDescription("List the page rules of a zone");
        pagerules.AddCommand<AddPageRuleCommand>("add")
            .WithDescription("Add a page rule, e.g. to force HTTPS on one hostname");
    });
});

return app.Run(args);
