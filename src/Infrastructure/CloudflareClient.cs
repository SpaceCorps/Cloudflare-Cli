using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

namespace Cloudflare.Console.Infrastructure;

public sealed class CloudflareClient
{
    private const string DefaultEndpoint = "https://api.cloudflare.com/client/v4/";

    private readonly HttpClient _http;

    public CloudflareClient(string apiToken, string? endpoint)
    {
        _http = new HttpClient { BaseAddress = new Uri(endpoint ?? DefaultEndpoint) };
        _http.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", apiToken);
        _http.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
    }

    public Task<object?> GetAsync(string path) => SendAsync(HttpMethod.Get, path, null);

    public Task<object?> PostAsync(string path, object body) => SendAsync(HttpMethod.Post, path, body);

    public Task<object?> PatchAsync(string path, object body) => SendAsync(HttpMethod.Patch, path, body);

    public Task<object?> PutAsync(string path, object body) => SendAsync(HttpMethod.Put, path, body);

    public Task<object?> DeleteAsync(string path) => SendAsync(HttpMethod.Delete, path, null);

    /// <summary>Returns the raw response body, for endpoints that do not return JSON (BIND export).</summary>
    public async Task<string> GetRawAsync(string path)
    {
        using var response = await _http.GetAsync(path);
        var text = await response.Content.ReadAsStringAsync();
        if (!response.IsSuccessStatusCode)
            throw new CloudflareApiException($"{(int)response.StatusCode} {response.ReasonPhrase}: {text}");
        return text;
    }

    /// <summary>Multipart upload, used by the BIND import endpoint.</summary>
    public async Task<object?> PostFileAsync(string path, string fileContent, IDictionary<string, string> fields)
    {
        using var form = new MultipartFormDataContent();
        var file = new StringContent(fileContent, Encoding.UTF8);
        file.Headers.ContentType = new MediaTypeHeaderValue("text/plain");
        form.Add(file, "file", "bind_config.txt");
        foreach (var (key, value) in fields)
            form.Add(new StringContent(value), key);

        using var response = await _http.PostAsync(path, form);
        return await ReadResultAsync(response);
    }

    private async Task<object?> SendAsync(HttpMethod method, string path, object? body)
    {
        using var request = new HttpRequestMessage(method, path);
        if (body is not null)
        {
            var json = JsonSerializer.Serialize(body);
            request.Content = new StringContent(json, Encoding.UTF8, "application/json");
        }

        using var response = await _http.SendAsync(request);
        return await ReadResultAsync(response);
    }

    private static async Task<object?> ReadResultAsync(HttpResponseMessage response)
    {
        var text = await response.Content.ReadAsStringAsync();

        JsonDocument document;
        try
        {
            document = JsonDocument.Parse(text);
        }
        catch (JsonException)
        {
            if (!response.IsSuccessStatusCode)
                throw new CloudflareApiException($"{(int)response.StatusCode} {response.ReasonPhrase}: {text}");
            return text;
        }

        using (document)
        {
            var root = document.RootElement;

            // Cloudflare wraps everything in {success, errors, messages, result}.
            if (root.ValueKind == JsonValueKind.Object &&
                root.TryGetProperty("success", out var success) &&
                success.ValueKind == JsonValueKind.False)
            {
                var details = root.TryGetProperty("errors", out var errors)
                    ? string.Join("; ", errors.EnumerateArray().Select(FormatError))
                    : text;
                throw new CloudflareApiException($"Cloudflare API error: {details}");
            }

            if (!response.IsSuccessStatusCode)
                throw new CloudflareApiException($"{(int)response.StatusCode} {response.ReasonPhrase}: {text}");

            return root.TryGetProperty("result", out var result)
                ? ToObject(result)
                : ToObject(root);
        }
    }

    private static string FormatError(JsonElement error)
    {
        var code = error.TryGetProperty("code", out var c) ? c.ToString() : "?";
        var message = error.TryGetProperty("message", out var m) ? m.GetString() : error.ToString();
        return $"[{code}] {message}";
    }

    /// <summary>Converts JSON into plain CLR types so YamlDotNet can serialize the result.</summary>
    private static object? ToObject(JsonElement element) => element.ValueKind switch
    {
        JsonValueKind.Object => element.EnumerateObject()
            .ToDictionary(p => p.Name, p => ToObject(p.Value)),
        JsonValueKind.Array => element.EnumerateArray().Select(ToObject).ToList(),
        JsonValueKind.String => element.GetString(),
        JsonValueKind.Number => element.TryGetInt64(out var l) ? l : element.GetDouble(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => null
    };

    /// <summary>
    /// Accepts either a zone id or a zone name and returns the id. Zone names are far easier to
    /// use from a runbook than 32-character hex ids.
    /// </summary>
    public async Task<string> ResolveZoneIdAsync(string zone)
    {
        if (zone.Length == 32 && zone.All(Uri.IsHexDigit))
            return zone;

        var result = await GetAsync($"zones?name={Uri.EscapeDataString(zone)}");
        if (result is List<object?> { Count: > 0 } list &&
            list[0] is Dictionary<string, object?> first &&
            first.TryGetValue("id", out var id) && id is string zoneId)
            return zoneId;

        throw new CloudflareApiException($"No zone found named '{zone}'.");
    }

    /// <summary>Finds a DNS record id by its full name, optionally filtered by type.</summary>
    public async Task<string> ResolveRecordIdAsync(string zoneId, string name, string? type)
    {
        var query = $"zones/{zoneId}/dns_records?name={Uri.EscapeDataString(name)}";
        if (!string.IsNullOrWhiteSpace(type)) query += $"&type={Uri.EscapeDataString(type)}";

        var result = await GetAsync(query);
        if (result is List<object?> list)
        {
            if (list.Count == 0)
                throw new CloudflareApiException($"No DNS record found named '{name}'.");
            if (list.Count > 1)
                throw new CloudflareApiException(
                    $"'{name}' matches {list.Count} records. Narrow it with --type, or use --id.");
            if (list[0] is Dictionary<string, object?> first &&
                first.TryGetValue("id", out var id) && id is string recordId)
                return recordId;
        }

        throw new CloudflareApiException($"No DNS record found named '{name}'.");
    }
}
