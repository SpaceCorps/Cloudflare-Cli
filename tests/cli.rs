//! Integration tests running the `cloudflare` binary against an in-process mock HTTP server.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct Recorded {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Value>,
    raw_body: Option<String>,
}

#[derive(Clone, Debug)]
enum MockResp {
    Json(Value),
    Raw(String),
}

type Route = (&'static str, &'static str, u16, MockResp);

struct Mock {
    url: String,
    log: Arc<Mutex<Vec<Recorded>>>,
}

impl Mock {
    fn start(routes: Vec<Route>) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/client/v4/", listener.local_addr().unwrap());
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let routes = routes.clone();
                let log = log2.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let mut parts = line.split_whitespace();
                    let method = parts.next().unwrap_or("").to_string();
                    let path = parts.next().unwrap_or("").trim_start_matches("/client/v4/").to_string();
                    let mut headers = Vec::new();
                    let mut len = 0usize;
                    loop {
                        let mut h = String::new();
                        reader.read_line(&mut h).unwrap();
                        let h = h.trim_end();
                        if h.is_empty() {
                            break;
                        }
                        if let Some((k, v)) = h.split_once(':') {
                            let (k, v) = (k.trim().to_lowercase(), v.trim().to_string());
                            if k == "content-length" {
                                len = v.parse().unwrap_or(0);
                            }
                            headers.push((k, v));
                        }
                    }
                    let mut buf = vec![0; len];
                    reader.read_exact(&mut buf).unwrap();
                    let (body, raw_body) = if len > 0 {
                        let parsed = serde_json::from_slice(&buf).ok();
                        let text = String::from_utf8_lossy(&buf).to_string();
                        (parsed, Some(text))
                    } else {
                        (None, None)
                    };
                    log.lock().unwrap().push(Recorded {
                        method: method.clone(),
                        path: path.clone(),
                        headers,
                        body,
                        raw_body,
                    });

                    let matched = routes
                        .iter()
                        .find(|(m, p, _, _)| *m == method && *p == path)
                        .map(|(_, _, s, b)| (*s, b.clone()));

                    let (status, resp_type) = matched.unwrap_or((
                        404,
                        MockResp::Json(json!({
                            "success": false,
                            "errors": [{"code": 7000, "message": "No route matched in mock"}]
                        })),
                    ));

                    match resp_type {
                        MockResp::Raw(text) => {
                            let _ = write!(
                                stream,
                                "HTTP/1.1 {status} OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                                text.len()
                            );
                        }
                        MockResp::Json(val) => {
                            let text = val.to_string();
                            let _ = write!(
                                stream,
                                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                                text.len()
                            );
                        }
                    }
                });
            }
        });
        Mock { url, log }
    }

    fn requests(&self) -> Vec<Recorded> {
        self.log.lock().unwrap().clone()
    }

    fn last(&self, method: &str) -> Recorded {
        self.requests().into_iter().rev().find(|r| r.method == method).expect("no such request")
    }
}

struct Env {
    dir: PathBuf,
    api: String,
}

impl Env {
    fn new(mock: &Mock) -> Env {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cloudflare-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Env { dir, api: mock.url.clone() }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cloudflare"))
            .args(args)
            .env("CLOUDFLARE_CONFIG_DIR", &self.dir)
            .env("CLOUDFLARE_SECRET_STORE", "plaintext")
            .env("CLOUDFLARE_ALLOW_PLAINTEXT_STORE", "1")
            .env("CLOUDFLARE_API_URL", &self.api)
            .output()
            .unwrap()
    }

    fn json(&self, args: &[&str]) -> (i32, Value, Value) {
        let mut all = args.to_vec();
        all.push("--json");
        let out = self.run(&all);
        let parse = |b: &[u8]| {
            let s = String::from_utf8_lossy(b);
            let s = s.lines().filter(|l| !l.starts_with("warning:")).collect::<Vec<_>>().join("\n");
            serde_json::from_str(&s).unwrap_or(Value::Null)
        };
        (out.status.code().unwrap_or(-1), parse(&out.stdout), parse(&out.stderr))
    }

    fn with_account(self) -> Env {
        let (code, out, err) = self.json(&["accounts", "add", "work", "--api-token", "cf_test_token"]);
        assert_eq!(code, 0, "{err}");
        assert_eq!(out["status"], "added");
        self
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn verify_route() -> Route {
    (
        "GET",
        "user/tokens/verify",
        200,
        MockResp::Json(json!({
            "success": true,
            "errors": [],
            "messages": [{"code": 10000, "message": "This API Token is valid and active"}],
            "result": {"id": "token_12345", "status": "active"}
        })),
    )
}

fn zone_lookup_route() -> Route {
    (
        "GET",
        "zones?name=example.com",
        200,
        MockResp::Json(json!({
            "success": true,
            "errors": [],
            "messages": [],
            "result": [{"id": "0123456789abcdef0123456789abcdef", "name": "example.com", "status": "active"}]
        })),
    )
}

#[test]
fn accounts_lifecycle() {
    let mock = Mock::start(vec![verify_route()]);
    let env = Env::new(&mock).with_account();

    assert_eq!(mock.last("GET").headers.iter().find(|(k, _)| k == "authorization").unwrap().1, "Bearer cf_test_token");

    let (code, out, _) = env.json(&["accounts", "list"]);
    assert_eq!(code, 0);
    assert_eq!(out["count"], 1);
    assert_eq!(out["accounts"][0]["identity"], "token_12345 (active)");
    assert_eq!(out["accounts"][0]["tokenStatus"], "stored");
    assert_eq!(out["secretStore"], "plaintext");

    let (_, out, _) = env.json(&["accounts", "list", "--check"]);
    assert_eq!(out["accounts"][0]["tokenStatus"], "valid");

    // Duplicate add without --force fails
    let (code, _, err) = env.json(&["accounts", "add", "work", "--api-token", "new_token"]);
    assert_eq!(code, 6);
    assert!(err["remediation"].as_str().unwrap().contains("--force"));

    // Test command
    let (code, out, _) = env.json(&["accounts", "test", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["tokenStatus"], "valid");

    // Remove without --yes
    let (code, _, err) = env.json(&["accounts", "remove", "work"]);
    assert_eq!(code, 6);
    assert_eq!(err["remediation"], "cloudflare accounts remove work --yes");

    // Remove with --yes
    let (code, out, _) = env.json(&["accounts", "remove", "work", "--yes"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "removed");

    let (_, out, _) = env.json(&["accounts", "list"]);
    assert_eq!(out["count"], 0);
}

#[test]
fn api_token_from_stdin() {
    let mock = Mock::start(vec![verify_route()]);
    let env = Env::new(&mock);
    let mut child = Command::new(env!("CARGO_BIN_EXE_cloudflare"))
        .args(["accounts", "add", "piped", "--api-token-stdin", "--json"])
        .env("CLOUDFLARE_CONFIG_DIR", &env.dir)
        .env("CLOUDFLARE_SECRET_STORE", "plaintext")
        .env("CLOUDFLARE_ALLOW_PLAINTEXT_STORE", "1")
        .env("CLOUDFLARE_API_URL", &env.api)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"cf_piped_token\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let auth = mock.last("GET").headers.into_iter().find(|(k, _)| k == "authorization").unwrap().1;
    assert_eq!(auth, "Bearer cf_piped_token");
}

#[test]
fn zones_commands() {
    let mock = Mock::start(vec![
        verify_route(),
        zone_lookup_route(),
        (
            "GET",
            "zones",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": [{"id": "z1", "name": "example.com"}]
            })),
        ),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "0123456789abcdef0123456789abcdef", "name": "example.com", "status": "active"}
            })),
        ),
        (
            "POST",
            "zones",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "z2", "name": "new.com"}
            })),
        ),
        (
            "POST",
            "zones/0123456789abcdef0123456789abcdef/purge_cache",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "0123456789abcdef0123456789abcdef"}
            })),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    // zones list
    let (code, out, _) = env.json(&["zones", "list", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out[0]["name"], "example.com");

    // zones get
    let (code, out, _) = env.json(&["zones", "get", "--zone", "example.com", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], "0123456789abcdef0123456789abcdef");

    // zones add
    let (code, out, _) = env.json(&["zones", "add", "--name", "new.com", "--account-id", "acc_1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["name"], "new.com");
    assert_eq!(mock.last("POST").body.unwrap()["account"]["id"], "acc_1");

    // zones purge-cache
    let (code, out, _) = env.json(&["zones", "purge-cache", "--zone", "example.com", "--everything", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "purged");
}

#[test]
fn records_crud_and_proxy() {
    let mock = Mock::start(vec![
        verify_route(),
        zone_lookup_route(),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/dns_records?per_page=100",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": [{"id": "rec_1", "name": "web.example.com", "type": "A", "content": "1.2.3.4"}]
            })),
        ),
        (
            "POST",
            "zones/0123456789abcdef0123456789abcdef/dns_records",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "rec_2", "name": "app.example.com", "type": "A", "content": "1.2.3.5"}
            })),
        ),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/dns_records?name=web.example.com",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": [{"id": "rec_1", "name": "web.example.com", "type": "A"}]
            })),
        ),
        (
            "PATCH",
            "zones/0123456789abcdef0123456789abcdef/dns_records/rec_1",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "rec_1", "content": "1.2.3.9"}
            })),
        ),
        (
            "DELETE",
            "zones/0123456789abcdef0123456789abcdef/dns_records/rec_1",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "rec_1"}
            })),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    // records list
    let (code, out, _) = env.json(&["records", "list", "--zone", "example.com", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out[0]["name"], "web.example.com");

    // records add
    let (code, out, _) = env.json(&[
        "records",
        "add",
        "--zone",
        "example.com",
        "--type",
        "A",
        "--name",
        "app.example.com",
        "--content",
        "1.2.3.5",
        "--proxied",
        "-a",
        "work",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], "rec_2");
    assert_eq!(mock.last("POST").body.unwrap()["proxied"], true);

    // records update by name
    let (code, out, _) = env.json(&[
        "records",
        "update",
        "--zone",
        "example.com",
        "--name",
        "web.example.com",
        "--content",
        "1.2.3.9",
        "-a",
        "work",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out["content"], "1.2.3.9");

    // records proxy on
    let (code, _, _) =
        env.json(&["records", "proxy", "--zone", "example.com", "--name", "web.example.com", "--on", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(mock.last("PATCH").body.unwrap()["proxied"], true);

    // records delete
    let (code, out, _) =
        env.json(&["records", "delete", "--zone", "example.com", "--name", "web.example.com", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "deleted");
}

#[test]
fn records_bind_export_and_import() {
    let mock = Mock::start(vec![
        verify_route(),
        zone_lookup_route(),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/dns_records/export",
            200,
            MockResp::Raw(";; BIND zone file\nexample.com. 300 IN A 1.2.3.4\n".into()),
        ),
        (
            "POST",
            "zones/0123456789abcdef0123456789abcdef/dns_records/import",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"recs_added": 1, "total_records_parsed": 1}
            })),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    // Export to stdout
    let out = env.run(&["records", "export", "--zone", "example.com", "-a", "work"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("BIND zone file"));

    // Import from file
    let tmp_bind = env.dir.join("zone.txt");
    std::fs::write(&tmp_bind, "example.com. 300 IN A 1.2.3.4\n").unwrap();
    let (code, out, _) =
        env.json(&["records", "import", "--zone", "example.com", "--file", tmp_bind.to_str().unwrap(), "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["recs_added"], 1);
}

#[test]
fn settings_and_ssl_status() {
    let mock = Mock::start(vec![
        verify_route(),
        zone_lookup_route(),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/settings",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": [{"id": "ssl", "value": "full"}, {"id": "always_use_https", "value": "off"}]
            })),
        ),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/settings/ssl",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "ssl", "value": "full"}
            })),
        ),
        (
            "PATCH",
            "zones/0123456789abcdef0123456789abcdef/settings/ssl",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "ssl", "value": "strict"}
            })),
        ),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/settings/always_use_https",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "always_use_https", "value": "on"}
            })),
        ),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/ssl/universal/settings",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"enabled": true}
            })),
        ),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/ssl/certificate_packs?status=all",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": [{"type": "universal", "status": "active", "hosts": ["example.com"]}]
            })),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    // settings list
    let (code, out, _) = env.json(&["settings", "list", "--zone", "example.com", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out[0]["id"], "ssl");

    // settings get
    let (code, out, _) = env.json(&["settings", "get", "--zone", "example.com", "--name", "ssl", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["value"], "full");

    // settings set
    let (code, out, _) =
        env.json(&["settings", "set", "--zone", "example.com", "--name", "ssl", "--value", "strict", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["value"], "strict");

    // ssl status
    let (code, out, _) = env.json(&["ssl", "status", "--zone", "example.com", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["encryption_mode"], "full");
    assert_eq!(out["always_use_https"], "on");
    assert_eq!(out["universal_ssl_enabled"], true);
    assert_eq!(out["certificate_packs"][0]["status"], "active");
}

#[test]
fn pagerules_crud() {
    let mock = Mock::start(vec![
        verify_route(),
        zone_lookup_route(),
        (
            "GET",
            "zones/0123456789abcdef0123456789abcdef/pagerules",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": [{"id": "pr_1", "status": "active"}]
            })),
        ),
        (
            "POST",
            "zones/0123456789abcdef0123456789abcdef/pagerules",
            200,
            MockResp::Json(json!({
                "success": true,
                "result": {"id": "pr_2", "status": "active"}
            })),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    // pagerules list
    let (code, out, _) = env.json(&["pagerules", "list", "--zone", "example.com", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out[0]["id"], "pr_1");

    // pagerules add
    let (code, out, _) = env.json(&[
        "pagerules",
        "add",
        "--zone",
        "example.com",
        "--url",
        "store.example.com/*",
        "--always-use-https",
        "-a",
        "work",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], "pr_2");
}

#[test]
fn http_errors_map_to_exit_codes() {
    let mock = Mock::start(vec![
        verify_route(),
        (
            "GET",
            "zones",
            403,
            MockResp::Json(json!({
                "success": false,
                "errors": [{"code": 10000, "message": "Authentication error"}]
            })),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    let (code, _, err) = env.json(&["zones", "list", "-a", "work"]);
    assert_eq!(code, 3);
    assert_eq!(err["code"], "auth_required");
    assert!(err["error"].as_str().unwrap().contains("Authentication error"));
}

#[test]
fn agent_readme_as_data_and_markdown() {
    let mock = Mock::start(vec![]);
    let env = Env::new(&mock);
    let (code, out, _) = env.json(&["agent-readme"]);
    assert_eq!(code, 0);
    assert_eq!(out["tool"], "cloudflare");
    assert_eq!(out["apiVersion"], "v4");
    assert_eq!(out["exitCodes"]["7"], "no_account - run cloudflare accounts list or login");

    let md = String::from_utf8(env.run(&["agent-readme"]).stdout).unwrap();
    assert!(md.starts_with("# cloudflare - agent operating manual"));
}
