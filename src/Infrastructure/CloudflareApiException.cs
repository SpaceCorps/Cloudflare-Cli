namespace Cloudflare.Console.Infrastructure;

public sealed class CloudflareApiException : Exception
{
    public CloudflareApiException(string message) : base(message) { }
}
