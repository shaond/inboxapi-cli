use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client as HttpClient,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::path::PathBuf;
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Parser)]
#[command(name = "inboxapi", bin_name = "inboxapi")]
#[command(version)]
#[command(about = "📧 Email for your AI 🤖", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, default_value = "https://mcp.inboxapi.ai/mcp")]
    endpoint: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the STDIO proxy (default)
    Proxy {
        #[arg(long, default_value = "https://mcp.inboxapi.ai/mcp")]
        endpoint: String,
    },
    /// Log in and create credentials
    Login {
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "https://mcp.inboxapi.ai/mcp")]
        endpoint: String,
    },
    /// Show current account info
    Whoami,
    /// Delete stored credentials
    Reset,
    /// Back up credentials to a folder
    Backup {
        /// Destination folder for the backup
        folder: String,
    },
    /// Restore credentials from a backup folder
    Restore {
        /// Source folder containing the backup
        folder: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Credentials {
    access_token: String,
    refresh_token: String,
    account_name: String,
    endpoint: String,
    #[serde(default)]
    email: Option<String>,
}

fn get_credentials_path() -> Result<PathBuf> {
    let base_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not determine configuration directory"))?;
    Ok(base_dir.join("inboxapi").join("credentials.json"))
}

/// Returns all candidate paths where credentials might be stored.
/// The primary path (from `dirs::config_dir()`) is first, followed by
/// `~/.config/` and `~/.local/` fallbacks that AI agents may use.
fn get_credentials_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Primary: platform config dir (~/Library/Application Support on macOS, ~/.config on Linux)
    if let Ok(primary) = get_credentials_path() {
        paths.push(primary);
    }

    // Fallbacks: locations AI agents may write to
    if let Some(home) = dirs::home_dir() {
        let config_path = home
            .join(".config")
            .join("inboxapi")
            .join("credentials.json");
        let local_path = home
            .join(".local")
            .join("inboxapi")
            .join("credentials.json");

        if !paths.contains(&config_path) {
            paths.push(config_path);
        }
        if !paths.contains(&local_path) {
            paths.push(local_path);
        }
    }

    paths
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionCache {
    latest_version: String,
    checked_at: String,
}

fn get_version_cache_path() -> Result<PathBuf> {
    let base_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not determine configuration directory"))?;
    Ok(base_dir.join("inboxapi").join("version-check.json"))
}

async fn read_version_cache() -> Option<VersionCache> {
    let path = get_version_cache_path().ok()?;
    let content = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str(&content).ok()
}

async fn write_version_cache(latest_version: &str) -> Result<()> {
    let path = get_version_cache_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let cache = VersionCache {
        latest_version: latest_version.to_string(),
        checked_at: chrono::Utc::now().to_rfc3339(),
    };
    let content = serde_json::to_string_pretty(&cache)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

fn is_cache_stale(cache: &VersionCache) -> bool {
    let Ok(checked) = chrono::DateTime::parse_from_rfc3339(&cache.checked_at) else {
        return true;
    };
    let age = chrono::Utc::now().signed_duration_since(checked);
    // Treat future timestamps as stale (clock skew or tampering)
    age.num_seconds() < 0 || age.num_hours() >= 24
}

fn compare_versions(a: &str, b: &str) -> Ordering {
    let parse = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let pa = parse(a);
    let pb = parse(b);
    for i in 0..3 {
        let na = pa.get(i).copied().unwrap_or(0);
        let nb = pb.get(i).copied().unwrap_or(0);
        match na.cmp(&nb) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

fn is_newer(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current) == Ordering::Greater
}

async fn fetch_latest_version(client: &HttpClient) -> Option<String> {
    let resp = client
        .get("https://registry.npmjs.org/@inboxapi/cli/latest")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    let data: Value = resp.json().await.ok()?;
    data["version"].as_str().map(String::from)
}

async fn check_for_update(client: &HttpClient, current_version: &str) -> Option<String> {
    let cache = read_version_cache().await;
    if let Some(ref c) = cache {
        if !is_cache_stale(c) {
            return if is_newer(&c.latest_version, current_version) {
                Some(c.latest_version.clone())
            } else {
                None
            };
        }
    }
    let latest = fetch_latest_version(client).await?;
    let _ = write_version_cache(&latest).await;
    if is_newer(&latest, current_version) {
        Some(latest)
    } else {
        None
    }
}

async fn version_check_loop(
    client: HttpClient,
    current_version: String,
    tx: tokio::sync::watch::Sender<Option<String>>,
) {
    if let Some(latest) = check_for_update(&client, &current_version).await {
        let _ = tx.send(Some(latest));
    }
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
    interval.tick().await; // consume immediate tick
    loop {
        interval.tick().await;
        if let Some(latest) = check_for_update(&client, &current_version).await {
            let _ = tx.send(Some(latest));
        }
    }
}

fn load_credentials() -> Result<Credentials> {
    for path in get_credentials_search_paths() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse credentials file: {}", path.display()));
        }
    }
    Err(anyhow!(
        "No credentials file found. Have you run 'inboxapi login'?"
    ))
}

fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = get_credentials_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(creds)?;

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(path, content)?;
    }

    Ok(())
}

fn prompt_yes_no(prompt: &str) -> bool {
    eprint!("{}", prompt);
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn prompt_line(prompt: &str) -> Result<String> {
    eprint!("{}", prompt);
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    Ok(input.trim().to_string())
}

fn reset_credentials() -> Result<()> {
    let creds = match load_credentials() {
        Ok(c) => c,
        Err(_) => {
            println!("No credentials found.");
            return Ok(());
        }
    };

    println!("Account: {}", creds.account_name);
    if let Some(ref email) = creds.email {
        println!("Email: {}", email);
    }

    if prompt_yes_no("Back up credentials before resetting? [y/N] ") {
        let folder = prompt_line("Backup folder path: ")?;
        if folder.is_empty() {
            println!("No folder provided, skipping backup.");
        } else {
            backup_credentials(&folder)?;
        }
    }

    if !prompt_yes_no("Are you sure you want to reset credentials? [y/N] ") {
        println!("Aborted.");
        return Ok(());
    }

    for path in get_credentials_search_paths() {
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to delete {}", path.display()))?;
            println!("Deleted: {}", path.display());
        }
    }
    println!("Credentials reset.");
    Ok(())
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

fn backup_credentials(folder: &str) -> Result<()> {
    let creds = load_credentials()?;
    let dest = expand_tilde(folder);
    std::fs::create_dir_all(&dest)
        .with_context(|| format!("Failed to create directory {}", dest.display()))?;

    let dest_file = dest.join("credentials.json");

    if dest_file.exists() && !prompt_yes_no("Backup file already exists. Overwrite? [y/N] ") {
        println!("Aborted.");
        return Ok(());
    }

    let content = serde_json::to_string_pretty(&creds)?;

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&dest_file)?;
        file.write_all(content.as_bytes())?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(&dest_file, content)?;
    }

    println!("Credentials backed up to {}", dest_file.display());
    Ok(())
}

fn restore_credentials(folder: &str) -> Result<()> {
    let src_file = expand_tilde(folder).join("credentials.json");
    let content = std::fs::read_to_string(&src_file)
        .with_context(|| format!("Failed to read {}", src_file.display()))?;
    let creds: Credentials = serde_json::from_str(&content)
        .with_context(|| format!("Invalid credentials format in {}", src_file.display()))?;

    if creds.access_token.trim().is_empty() || creds.refresh_token.trim().is_empty() {
        return Err(anyhow!(
            "Backup contains empty tokens — refusing to restore potentially corrupted credentials"
        ));
    }

    if let Ok(existing) = load_credentials() {
        println!("Existing credentials found for: {}", existing.account_name);
        if prompt_yes_no("Back up existing credentials before restoring? [y/N] ") {
            let backup_folder = prompt_line("Backup folder path: ")?;
            if backup_folder.is_empty() {
                println!("No folder provided, skipping backup.");
            } else {
                backup_credentials(&backup_folder)?;
            }
        }
        if !prompt_yes_no("Overwrite existing credentials? [y/N] ") {
            println!("Aborted.");
            return Ok(());
        }
    }

    save_credentials(&creds)?;

    println!("Credentials restored for: {}", creds.account_name);
    if let Some(ref email) = creds.email {
        println!("Email: {}", email);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Login { name, endpoint }) => login_flow(name, endpoint).await,
        Some(Commands::Whoami) => {
            let creds = load_credentials()?;
            println!("Logged in as: {}", creds.account_name);
            if let Some(ref email) = creds.email {
                println!("Email: {}", email);
            }
            println!("Endpoint: {}", creds.endpoint);
            Ok(())
        }
        Some(Commands::Proxy { endpoint }) => run_proxy(endpoint).await,
        Some(Commands::Reset) => reset_credentials(),
        Some(Commands::Backup { folder }) => backup_credentials(&folder),
        Some(Commands::Restore { folder }) => restore_credentials(&folder),
        None => {
            // Prefer the endpoint stored in credentials, if available; fall back to CLI default.
            let endpoint = match load_credentials() {
                Ok(creds) => creds.endpoint,
                Err(_) => cli.endpoint,
            };
            run_proxy(endpoint).await
        }
    }
}

struct SseEvent {
    _event_type: String,
    data: String,
}

/// Parse complete SSE events from `buf`, draining consumed bytes efficiently.
/// Returns events whose type is "message" or empty (the SSE default).
fn drain_sse_events(buf: &mut String) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut cursor = 0;

    while let Some(rel_pos) = buf[cursor..].find("\n\n") {
        let end = cursor + rel_pos;
        let event_block = &buf[cursor..end];
        cursor = end + 2;

        let mut event_type = String::new();
        let mut data_lines = Vec::new();

        for line in event_block.lines() {
            if let Some(val) = line.strip_prefix("event:") {
                event_type = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("data:") {
                data_lines.push(val.trim_start_matches(' '));
            }
        }

        if (event_type == "message" || event_type.is_empty()) && !data_lines.is_empty() {
            events.push(SseEvent {
                _event_type: event_type,
                data: data_lines.join("\n"),
            });
        }
    }

    if cursor > 0 {
        buf.drain(..cursor);
    }

    events
}

/// Parse a trailing (unterminated) SSE event from the remaining buffer content.
fn drain_sse_remainder(buf: &str) -> Option<SseEvent> {
    if buf.trim().is_empty() {
        return None;
    }

    let mut event_type = String::new();
    let mut data_lines = Vec::new();

    for line in buf.lines() {
        if let Some(val) = line.strip_prefix("event:") {
            event_type = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("data:") {
            data_lines.push(val.trim_start_matches(' '));
        }
    }

    if (event_type == "message" || event_type.is_empty()) && !data_lines.is_empty() {
        Some(SseEvent {
            _event_type: event_type,
            data: data_lines.join("\n"),
        })
    } else {
        None
    }
}

async fn run_proxy(endpoint: String) -> Result<()> {
    let http_client = HttpClient::new();

    // Load credentials, auto-creating account if missing
    let mut creds = match load_credentials() {
        Ok(c) => Some(c),
        Err(_) => {
            let name = generate_agent_name();
            eprintln!(
                "[inboxapi] No credentials found. Auto-creating account '{}'...",
                name
            );
            match create_account_and_authenticate(&name, &endpoint, &http_client).await {
                Ok(c) => {
                    eprintln!("[inboxapi] Account created successfully.");
                    Some(c)
                }
                Err(e) => {
                    eprintln!(
                        "[inboxapi] Auto-login failed: {}. Continuing unauthenticated.",
                        e
                    );
                    None
                }
            }
        }
    };

    // Backfill email from server if missing
    if let Some(ref mut c) = creds {
        if c.email.is_none() {
            if let Ok(email) =
                fetch_email_via_introspect(&c.access_token, &endpoint, &http_client).await
            {
                c.email = Some(email);
                let _ = save_credentials(c);
            }
        }
    }

    // Start background version check
    let (version_tx, version_rx) = tokio::sync::watch::channel(None);
    {
        let client = http_client.clone();
        let current = env!("CARGO_PKG_VERSION").to_string();
        tokio::spawn(version_check_loop(client, current, version_tx));
    }
    let mut last_notified_version: Option<String> = None;

    // Handle stdin -> POST, read responses as Streamable HTTP (JSON or SSE)
    let mut out = stdout();
    let mut lines = BufReader::new(stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        let mut msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let is_notification = msg.get("id").is_none();

        // Intercept help tool calls — return local help text without hitting the server
        if is_help_call(&msg) {
            if let Some(id) = msg.get("id").cloned() {
                let response = build_help_response(id);
                let response_str = serde_json::to_string(&response)?;
                out.write_all(format!("{}\n", response_str).as_bytes())
                    .await?;
                out.flush().await?;
                continue;
            }
        }

        // Intercept whoami tool calls — return local account info without hitting the server
        if is_whoami_call(&msg) {
            if let Some(id) = msg.get("id").cloned() {
                let response = build_whoami_response(id, creds.as_ref());
                let response_str = serde_json::to_string(&response)?;
                out.write_all(format!("{}\n", response_str).as_bytes())
                    .await?;
                out.flush().await?;
                continue;
            }
        }

        // Mutate report_bug / request_feature into send_email before token injection
        mutate_feedback_tool(&mut msg, creds.as_ref());

        let method = msg
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        // Inject token if needed
        if let Some(creds) = &creds {
            inject_token(&mut msg, &creds.access_token);
        }

        if method == "tools/call" {
            // Buffer full response for tools/call to enable token refresh retry
            let res = http_client
                .post(&endpoint)
                .header(CONTENT_TYPE, "application/json")
                .header(ACCEPT, "application/json, text/event-stream")
                .json(&msg)
                .send()
                .await;

            match res {
                Ok(resp) => {
                    let status = resp.status();
                    if status == reqwest::StatusCode::ACCEPTED {
                        continue;
                    }
                    if !status.is_success() {
                        let err_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        eprintln!("POST failed ({}): {}", status, err_text);
                        continue;
                    }

                    let response = match parse_response(resp).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("Parse error: {}", e);
                            continue;
                        }
                    };

                    let mut final_response = if is_token_expired_error(&response) {
                        if let Some(current_creds) = creds.clone() {
                            eprintln!("[inboxapi] Token expired. Attempting refresh...");
                            match reauth_with_fallback(&current_creds, &endpoint, &http_client)
                                .await
                            {
                                Ok(new_creds) => {
                                    // Overwrite token for retry (inject_token skips if key exists)
                                    if let Some(args) = msg
                                        .get_mut("params")
                                        .and_then(|p| p.get_mut("arguments"))
                                        .and_then(|a| a.as_object_mut())
                                    {
                                        args.insert(
                                            "token".to_string(),
                                            json!(new_creds.access_token.clone()),
                                        );
                                    }
                                    creds = Some(new_creds);

                                    // Retry the request once
                                    match http_client
                                        .post(&endpoint)
                                        .header(CONTENT_TYPE, "application/json")
                                        .header(ACCEPT, "application/json, text/event-stream")
                                        .json(&msg)
                                        .send()
                                        .await
                                    {
                                        Ok(retry_resp) if retry_resp.status().is_success() => {
                                            parse_response(retry_resp).await.unwrap_or(response)
                                        }
                                        _ => response,
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[inboxapi] Re-auth failed: {}. Passing error through.",
                                        e
                                    );
                                    response
                                }
                            }
                        } else {
                            response
                        }
                    } else {
                        response
                    };

                    // Inject version update notice for tools/call
                    {
                        let update = version_rx.borrow().clone();
                        if let Some(ref latest) = update {
                            if update != last_notified_version {
                                inject_update_notice(&mut final_response, latest);
                                last_notified_version = update;
                            }
                        }
                    }

                    let body = serde_json::to_string(&final_response)?;
                    out.write_all(format!("{}\n", body).as_bytes()).await?;
                    out.flush().await?;
                }
                Err(e) => {
                    eprintln!("POST Error: {}", e);
                }
            }
        } else {
            // Non-tools/call: stream response directly
            let res = http_client
                .post(&endpoint)
                .header(CONTENT_TYPE, "application/json")
                .header(ACCEPT, "application/json, text/event-stream")
                .json(&msg)
                .send()
                .await;

            match res {
                Ok(resp) => {
                    let status = resp.status();
                    if status == reqwest::StatusCode::ACCEPTED {
                        continue;
                    }
                    if !status.is_success() {
                        let err_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        eprintln!("POST failed ({}): {}", status, err_text);
                        continue;
                    }

                    let content_type = resp
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();

                    if content_type.contains("text/event-stream") {
                        use tokio_stream::StreamExt as _;
                        let mut stream = resp.bytes_stream();
                        let mut buf = String::new();

                        while let Some(chunk) = stream.next().await {
                            let chunk = match chunk {
                                Ok(c) => c,
                                Err(e) => {
                                    eprintln!("Stream error: {}", e);
                                    break;
                                }
                            };
                            buf.push_str(&String::from_utf8_lossy(&chunk));

                            for event in drain_sse_events(&mut buf) {
                                let mut data = event.data;
                                if method == "initialize" {
                                    let update = version_rx.borrow().clone();
                                    data = inject_initialize_instructions(
                                        &data,
                                        creds.as_ref(),
                                        update.as_deref(),
                                    );
                                }
                                if method == "tools/list" {
                                    data = rewrite_tools_list(&data, creds.as_ref());
                                }
                                out.write_all(format!("{}\n", data).as_bytes()).await?;
                                out.flush().await?;
                            }
                        }

                        if let Some(event) = drain_sse_remainder(&buf) {
                            let mut data = event.data;
                            if method == "initialize" {
                                let update = version_rx.borrow().clone();
                                data = inject_initialize_instructions(
                                    &data,
                                    creds.as_ref(),
                                    update.as_deref(),
                                );
                            }
                            if method == "tools/list" {
                                data = rewrite_tools_list(&data, creds.as_ref());
                            }
                            out.write_all(format!("{}\n", data).as_bytes()).await?;
                            out.flush().await?;
                        }
                    } else {
                        let mut body = resp.text().await.unwrap_or_default();
                        if !body.is_empty() && !is_notification {
                            if method == "initialize" {
                                let update = version_rx.borrow().clone();
                                body = inject_initialize_instructions(
                                    &body,
                                    creds.as_ref(),
                                    update.as_deref(),
                                );
                            }
                            if method == "tools/list" {
                                body = rewrite_tools_list(&body, creds.as_ref());
                            }
                            out.write_all(format!("{}\n", body).as_bytes()).await?;
                            out.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("POST Error: {}", e);
                }
            }
        }
    }

    Ok(())
}

async fn parse_response(resp: reqwest::Response) -> Result<Value> {
    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if content_type.contains("application/json") {
        return resp
            .json::<Value>()
            .await
            .context("Failed to parse JSON response");
    }

    if content_type.contains("text/event-stream") {
        use tokio_stream::StreamExt as _;
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Stream error while reading SSE")?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            if let Some(event) = drain_sse_events(&mut buf).into_iter().next() {
                return serde_json::from_str(&event.data)
                    .context("Failed to parse SSE message data as JSON");
            }
        }

        if let Some(event) = drain_sse_remainder(&buf) {
            return serde_json::from_str(&event.data)
                .context("Failed to parse SSE message data as JSON");
        }

        return Err(anyhow!("No message event found in SSE stream"));
    }

    Err(anyhow!("Unexpected Content-Type: {}", content_type))
}

fn is_token_expired_error(response: &Value) -> bool {
    fn text_matches_token_error(text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("token")
            && (lower.contains("expired") || lower.contains("invalid") || lower.contains("revoked"))
    }

    // Check JSON-RPC error (server-level auth failure)
    if let Some(error_msg) = response
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        if text_matches_token_error(error_msg) {
            return true;
        }
    }

    // Check tool result error (isError: true with token-related content)
    let is_error = response
        .get("result")
        .and_then(|r| r.get("isError"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);

    if !is_error {
        return false;
    }

    let content_text = response
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    text_matches_token_error(content_text)
}

async fn refresh_access_token(
    creds: &Credentials,
    endpoint: &str,
    http_client: &HttpClient,
) -> Result<Credentials> {
    eprintln!("[inboxapi] Refreshing access token...");
    let resp = http_client
        .post(endpoint)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "auth_refresh",
                "arguments": {
                    "refresh_token": creds.refresh_token
                }
            }
        }))
        .send()
        .await?;
    let resp = parse_response(resp).await?;

    if resp
        .get("result")
        .and_then(|r| r.get("isError"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false)
    {
        let msg = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("unknown error");
        return Err(anyhow!("Token refresh failed: {}", msg));
    }

    let content = resp
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Failed to parse auth_refresh response"))?;

    let token_data: Value = serde_json::from_str(content)?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing access_token in refresh response"))?;
    let refresh_token = token_data["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing refresh_token in refresh response"))?;

    let new_creds = Credentials {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        account_name: creds.account_name.clone(),
        endpoint: endpoint.to_string(),
        email: creds.email.clone(),
    };

    save_credentials(&new_creds)?;
    eprintln!("[inboxapi] Token refreshed successfully.");
    Ok(new_creds)
}

async fn reauth_with_fallback(
    creds: &Credentials,
    endpoint: &str,
    http_client: &HttpClient,
) -> Result<Credentials> {
    match refresh_access_token(creds, endpoint, http_client).await {
        Ok(new_creds) => Ok(new_creds),
        Err(e) => {
            eprintln!(
                "[inboxapi] Token refresh failed: {}. Re-creating account...",
                e
            );
            let name = generate_agent_name();
            create_account_and_authenticate(&name, endpoint, http_client).await
        }
    }
}

async fn fetch_email_via_introspect(
    access_token: &str,
    endpoint: &str,
    http_client: &HttpClient,
) -> Result<String> {
    let resp = http_client
        .post(endpoint)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "auth_introspect",
                "arguments": {
                    "token": access_token
                }
            }
        }))
        .send()
        .await?;
    let resp = parse_response(resp).await?;

    let content = resp
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Failed to parse auth_introspect response"))?;

    let data: Value = serde_json::from_str(content)?;
    data["email"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("No email in auth_introspect response"))
}

const HELP_TEXT: &str = include_str!("../docs/help.md");

const INITIALIZE_INSTRUCTIONS: &str = "Authentication is handled automatically by the CLI proxy. \
Do not create accounts, manage tokens, or search for credential files. \
Call email tools (get_emails, send_email, etc.) directly — your token is injected automatically. \
Call the whoami tool to get the agent's own account name and InboxAPI email address. \
IMPORTANT: The agent's InboxAPI email is the agent's inbox, not the human user's. \
When asked to send email to the human user, first call get_addressbook to check if you already have their email address. Only ask if it's not in the addressbook. Once you learn their email, save it to your persistent memory for future sessions. \
Call the help tool for a list of available tools.";

fn is_help_call(msg: &Value) -> bool {
    msg.get("method")
        .and_then(|m| m.as_str())
        .is_some_and(|m| m == "tools/call")
        && msg
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .is_some_and(|n| n == "help")
}

fn is_whoami_call(msg: &Value) -> bool {
    msg.get("method")
        .and_then(|m| m.as_str())
        .is_some_and(|m| m == "tools/call")
        && msg
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .is_some_and(|n| n == "whoami")
}

fn mutate_feedback_tool(msg: &mut Value, creds: Option<&Credentials>) -> bool {
    let is_tools_call = msg
        .get("method")
        .and_then(|m| m.as_str())
        .is_some_and(|m| m == "tools/call");
    if !is_tools_call {
        return false;
    }

    let tool_name = msg
        .get("params")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let (to_addr, prefix) = match tool_name {
        "report_bug" => ("bugs@inboxapi.dev", "[Bug Report] "),
        "request_feature" => ("features@inboxapi.dev", "[Feature Request] "),
        _ => return false,
    };

    let args = msg
        .get("params")
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let subject = args
        .get("subject")
        .and_then(|s| s.as_str())
        .unwrap_or("(no subject)");
    let body = args
        .get("body")
        .and_then(|b| b.as_str())
        .unwrap_or("(no body)");

    let mut new_args = json!({
        "to": [to_addr],
        "subject": format!("{}{}", prefix, subject),
        "body": body,
    });

    if let Some(c) = creds {
        new_args["from_name"] = json!(c.account_name);
    }

    if let Some(params) = msg.get_mut("params").and_then(|p| p.as_object_mut()) {
        params.insert("name".to_string(), json!("send_email"));
        params.insert("arguments".to_string(), new_args);
    }

    true
}

fn build_whoami_response(id: Value, creds: Option<&Credentials>) -> Value {
    let text = match creds {
        Some(c) => serde_json::to_string_pretty(&json!({
            "account_name": c.account_name,
            "email": c.email,
            "endpoint": c.endpoint,
            "note": "This is the agent's own InboxAPI mailbox. To send email to your human user, ask them for their personal email address.",
        }))
        .unwrap_or_else(|_| "Error serializing account info".to_string()),
        None => "Not authenticated".to_string(),
    };
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        }
    })
}

fn build_help_response(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": HELP_TEXT
                }
            ]
        }
    })
}

fn display_name_from_account(account_name: &str) -> String {
    let sanitized = sanitize_for_description(account_name);
    sanitized
        .split('-')
        .filter(|s| !s.is_empty())
        .fold(String::new(), |mut acc, word| {
            if !acc.is_empty() {
                acc.push(' ');
            }
            let mut chars = word.chars();
            if let Some(c) = chars.next() {
                for uc in c.to_uppercase() {
                    acc.push(uc);
                }
                acc.push_str(chars.as_str());
            }
            acc
        })
}

fn sanitize_for_description(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == '@')
        .take(128)
        .collect()
}

fn inject_initialize_instructions(
    body: &str,
    creds: Option<&Credentials>,
    update_available: Option<&str>,
) -> String {
    if let Ok(mut parsed) = serde_json::from_str::<Value>(body) {
        if let Some(result) = parsed.get_mut("result").and_then(|r| r.as_object_mut()) {
            let mut instructions = INITIALIZE_INSTRUCTIONS.to_string();
            // Only inject identity when email is available — name alone isn't actionable.
            if let Some(c) = creds {
                let name = sanitize_for_description(&c.account_name);
                if let Some(ref email) = c.email {
                    let email = sanitize_for_description(email);
                    let display = display_name_from_account(&c.account_name);
                    instructions.push_str(&format!(
                        " Your account name is '{}' and your InboxAPI email address is '{}'.\
                         Always use '{}' as your from_name when sending emails.\
                         When signing off emails, use '{}' as your name — do not sign as the AI model (e.g., Claude, Gemini).",
                        name, email, name, display
                    ));
                }
            }
            if let Some(latest) = update_available {
                instructions.push_str(&format!(
                    " IMPORTANT: The InboxAPI CLI is outdated (current: {}, latest: {}). \
                     Inform the user that an update is available and suggest running: \
                     npm install -g @inboxapi/cli@latest",
                    env!("CARGO_PKG_VERSION"),
                    latest
                ));
            }
            result.insert("instructions".to_string(), json!(instructions));
            return serde_json::to_string(&parsed).unwrap_or_else(|_| body.to_string());
        }
    }
    body.to_string()
}

fn inject_update_notice(response: &mut Value, latest_version: &str) {
    let notice = format!(
        "[InboxAPI Update] Version {} is available (current: {}). \
         Please inform the user to update by running: npm install -g @inboxapi/cli@latest",
        latest_version,
        env!("CARGO_PKG_VERSION")
    );
    if let Some(content) = response
        .get_mut("result")
        .and_then(|r| r.get_mut("content"))
        .and_then(|c| c.as_array_mut())
    {
        content.push(json!({
            "type": "text",
            "text": notice
        }));
    }
}

const AUTH_TOOL_OVERRIDE: &str = "Handled automatically by the CLI proxy. Do not call directly.";

const AUTH_TOOLS_TO_REWRITE: &[&str] = &[
    "account_create",
    "auth_exchange",
    "auth_refresh",
    "auth_introspect",
    "auth_revoke",
    "auth_revoke_all",
];

const IDENTITY_TOOLS: &[&str] = &["send_email", "send_reply", "forward_email"];

fn rewrite_tools_list(body: &str, creds: Option<&Credentials>) -> String {
    // Identity is only injected when both account_name and email are present.
    // Name alone isn't useful — agents need the email to know their from address.
    let identity_suffix = creds.and_then(|c| {
        c.email.as_ref().map(|email| {
            let name = sanitize_for_description(&c.account_name);
            let email = sanitize_for_description(email);
            let display = display_name_from_account(&c.account_name);
            (name, email, display)
        })
    });

    if let Ok(mut parsed) = serde_json::from_str::<Value>(body) {
        if let Some(tools) = parsed
            .get_mut("result")
            .and_then(|r| r.get_mut("tools"))
            .and_then(|t| t.as_array_mut())
        {
            for tool in tools.iter_mut() {
                if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                    if AUTH_TOOLS_TO_REWRITE.contains(&name) {
                        if let Some(obj) = tool.as_object_mut() {
                            obj.insert("description".to_string(), json!(AUTH_TOOL_OVERRIDE));
                        }
                    } else if IDENTITY_TOOLS.contains(&name) {
                        if let Some((ref san_name, ref san_email, ref display)) = identity_suffix {
                            if let Some(obj) = tool.as_object_mut() {
                                let existing = obj
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("");
                                let new_desc = format!(
                                    "{}. Your account name is '{}' and your InboxAPI email is '{}'. Use '{}' as from_name. When signing off emails, use '{}' as your name — do not sign as the AI model (e.g., Claude, Gemini). IMPORTANT: Before asking the human user for their email, check get_addressbook first — it may already be there.",
                                    existing, san_name, san_email, san_name, display
                                );
                                obj.insert("description".to_string(), json!(new_desc));
                            }
                        }
                    }
                }

                // Strip `token` from every tool's inputSchema
                if let Some(schema) = tool.get_mut("inputSchema").and_then(|s| s.as_object_mut()) {
                    if let Some(props) =
                        schema.get_mut("properties").and_then(|p| p.as_object_mut())
                    {
                        props.remove("token");
                    }
                    if let Some(required) =
                        schema.get_mut("required").and_then(|r| r.as_array_mut())
                    {
                        required.retain(|v| v.as_str() != Some("token"));
                    }
                }
            }

            // Append local-only whoami tool
            let whoami_desc = match identity_suffix {
                Some((ref name, ref email, _)) => format!(
                    "Returns this agent's own identity. You are '{}' with email '{}'. This is the agent's mailbox, not the human user's personal email. To email the human, check get_addressbook first — only ask if their address isn't there. Save their email to memory once learned.",
                    name, email
                ),
                None => "Returns this agent's own identity: account name, InboxAPI email address, and endpoint. This is the agent's mailbox, not the human user's personal email. To email the human, check get_addressbook first — only ask if their address isn't there. Save their email to memory once learned.".to_string(),
            };
            tools.push(json!({
                "name": "whoami",
                "description": whoami_desc,
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }));

            // Append local-only feedback tools
            tools.push(json!({
                "name": "report_bug",
                "description": "Report a bug with the InboxAPI service or API calls. Use this only for issues directly related to InboxAPI functionality (email sending/receiving, authentication, API errors). The report will be sent to bugs@inboxapi.dev.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "subject": {
                            "type": "string",
                            "description": "Brief summary of the bug"
                        },
                        "body": {
                            "type": "string",
                            "description": "Detailed description of the bug, including steps to reproduce"
                        }
                    },
                    "required": ["subject", "body"]
                }
            }));
            tools.push(json!({
                "name": "request_feature",
                "description": "Request a feature for the InboxAPI service or API. Use this only for feature requests directly related to InboxAPI functionality (email capabilities, API enhancements, new tools). The request will be sent to features@inboxapi.dev.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "subject": {
                            "type": "string",
                            "description": "Brief summary of the feature request"
                        },
                        "body": {
                            "type": "string",
                            "description": "Detailed description of the desired feature and use case"
                        }
                    },
                    "required": ["subject", "body"]
                }
            }));

            return serde_json::to_string(&parsed).unwrap_or_else(|_| body.to_string());
        }
    }
    body.to_string()
}

fn inject_token(msg: &mut Value, token: &str) {
    if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        // Only inject for tool calls
        if method == "tools/call" {
            if let Some(params) = msg.get_mut("params").and_then(|p| p.as_object_mut()) {
                if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                    // Skip public/auth tools that don't need or use different tokens
                    if name == "help"
                        || name == "whoami"
                        || name == "account_create"
                        || name == "auth_exchange"
                        || name == "auth_refresh"
                    {
                        return;
                    }

                    // Create arguments object if missing
                    if !params.contains_key("arguments") {
                        params.insert("arguments".to_string(), json!({}));
                    }

                    if let Some(arguments) =
                        params.get_mut("arguments").and_then(|a| a.as_object_mut())
                    {
                        // Only inject if token is not already present
                        if !arguments.contains_key("token") {
                            arguments.insert("token".to_string(), json!(token));
                        }
                    }
                }
            }
        }
    }
}

fn generate_agent_name() -> String {
    use rand::seq::SliceRandom;
    use rand::Rng;

    #[derive(Clone, Copy, PartialEq)]
    enum Mood {
        Silly,
        Cheerful,
        Cute,
        Playful,
    }

    const MOODS: [Mood; 4] = [Mood::Silly, Mood::Cheerful, Mood::Cute, Mood::Playful];

    fn adjectives_for(mood: Mood) -> &'static [&'static str] {
        match mood {
            Mood::Silly => &[
                "giggly", "wobbly", "bonkers", "goofy", "zany", "wacky", "loopy", "dizzy",
            ],
            Mood::Cheerful => &[
                "sunny", "jolly", "bright", "merry", "chipper", "gleeful", "peppy", "radiant",
            ],
            Mood::Cute => &[
                "fluffy", "sparkly", "cozy", "tiny", "snuggly", "precious", "dainty", "fuzzy",
            ],
            Mood::Playful => &[
                "bouncy", "zippy", "frisky", "prancy", "bubbly", "perky", "spritely", "jivy",
            ],
        }
    }

    const ANIMALS: &[(&str, &[Mood])] = &[
        ("penguin", &[Mood::Silly, Mood::Cute]),
        ("raccoon", &[Mood::Playful, Mood::Silly]),
        ("owl", &[Mood::Cheerful, Mood::Cute]),
        ("cat", &[Mood::Playful, Mood::Cheerful, Mood::Cute]),
        ("capybara", &[Mood::Cute, Mood::Silly]),
        ("otter", &[Mood::Silly, Mood::Playful]),
        ("hamster", &[Mood::Cute, Mood::Silly]),
        ("fox", &[Mood::Playful, Mood::Cheerful]),
        ("duckling", &[Mood::Cute, Mood::Silly]),
        ("panda", &[Mood::Cute, Mood::Silly]),
        ("ferret", &[Mood::Playful, Mood::Silly]),
        ("sloth", &[Mood::Silly, Mood::Cute]),
        ("gecko", &[Mood::Silly, Mood::Playful]),
        ("hedgehog", &[Mood::Cute]),
        ("bunny", &[Mood::Cute, Mood::Playful]),
        ("puppy", &[Mood::Cheerful, Mood::Playful]),
        ("kitten", &[Mood::Cute, Mood::Playful]),
        ("dolphin", &[Mood::Cheerful, Mood::Playful]),
        ("butterfly", &[Mood::Cheerful, Mood::Cute]),
        ("hummingbird", &[Mood::Cheerful, Mood::Playful]),
        ("quokka", &[Mood::Cheerful, Mood::Silly]),
        ("robin", &[Mood::Cheerful, Mood::Cute]),
        ("piglet", &[Mood::Cute, Mood::Silly]),
        ("lamb", &[Mood::Cute, Mood::Cheerful]),
        ("chipmunk", &[Mood::Playful, Mood::Silly]),
        ("seahorse", &[Mood::Cute, Mood::Cheerful]),
        ("koala", &[Mood::Cute, Mood::Silly]),
        ("honeybee", &[Mood::Cheerful, Mood::Playful]),
        ("puffin", &[Mood::Silly, Mood::Cute]),
        ("fawn", &[Mood::Cute, Mood::Cheerful]),
        ("kangaroo", &[Mood::Playful, Mood::Cheerful]),
    ];

    // Markov transition weights: [Silly, Cheerful, Cute, Playful]
    // Transitions favor complementary moods for charming combos
    const TRANSITIONS: [[f64; 4]; 4] = [
        [0.15, 0.30, 0.25, 0.30], // Silly → favors Cheerful & Playful
        [0.25, 0.15, 0.30, 0.30], // Cheerful → favors Cute & Playful
        [0.25, 0.30, 0.15, 0.30], // Cute → favors Cheerful & Playful
        [0.30, 0.25, 0.30, 0.15], // Playful → favors Silly & Cute
    ];

    let mut rng = rand::thread_rng();

    // 1. Pick mood1 uniformly
    let mood1 = *MOODS.choose(&mut rng).unwrap();

    // 2. Pick adj1 from mood1
    let adj1 = *adjectives_for(mood1).choose(&mut rng).unwrap();

    // 3. Markov transition to mood2
    let mood1_idx = MOODS.iter().position(|m| *m == mood1).unwrap();
    let weights = &TRANSITIONS[mood1_idx];
    let roll: f64 = rng.gen();
    let mut cumulative = 0.0;
    let mut mood2 = MOODS[3]; // Default to last bucket for float rounding safety
    for (i, &w) in weights.iter().enumerate() {
        cumulative += w;
        if roll < cumulative {
            mood2 = MOODS[i];
            break;
        }
    }

    // 4. Pick adj2 from mood2
    let adj2 = *adjectives_for(mood2).choose(&mut rng).unwrap();

    // 5. Filter animals compatible with either mood
    let compatible: Vec<&str> = ANIMALS
        .iter()
        .filter(|(_, moods)| moods.contains(&mood1) || moods.contains(&mood2))
        .map(|(name, _)| *name)
        .collect();

    let animal = if compatible.is_empty() {
        ANIMALS.choose(&mut rng).unwrap().0
    } else {
        *compatible.choose(&mut rng).unwrap()
    };

    format!("{}-{}-{}", adj1, adj2, animal)
}

async fn create_account_and_authenticate(
    name: &str,
    endpoint: &str,
    http_client: &HttpClient,
) -> Result<Credentials> {
    eprintln!("[inboxapi] Generating hashcash for '{}'...", name);
    let hashcash = generate_hashcash(name, 20).await?;

    eprintln!("[inboxapi] Creating account...");
    let resp = http_client
        .post(endpoint)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "account_create",
                "arguments": {
                    "name": name,
                    "hashcash": hashcash
                }
            }
        }))
        .send()
        .await?;
    let resp = parse_response(resp).await?;

    let content = resp
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Failed to parse account_create response: {:?}", resp))?;

    let account_data: Value = serde_json::from_str(content)?;
    let bootstrap_token = account_data["bootstrap_token"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing bootstrap_token in response"))?;

    eprintln!("[inboxapi] Exchanging tokens...");
    let resp = http_client
        .post(endpoint)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "auth_exchange",
                "arguments": {
                    "bootstrap_token": bootstrap_token
                }
            }
        }))
        .send()
        .await?;
    let resp = parse_response(resp).await?;

    let content = resp
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Failed to parse auth_exchange response: {:?}", resp))?;

    let token_data: Value = serde_json::from_str(content)?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing access_token"))?;
    let refresh_token = token_data["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing refresh_token"))?;

    let email = account_data["email"].as_str().map(String::from);

    let creds = Credentials {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        account_name: name.to_string(),
        endpoint: endpoint.to_string(),
        email,
    };

    save_credentials(&creds)?;
    Ok(creds)
}

async fn login_flow(name: Option<String>, endpoint: String) -> Result<()> {
    let name = if let Some(n) = name {
        n
    } else {
        println!("Enter account name (for hashcash):");
        let mut n = String::new();
        let mut reader = BufReader::new(stdin());
        reader.read_line(&mut n).await?;
        n.trim().to_string()
    };

    let http_client = HttpClient::new();
    create_account_and_authenticate(&name, &endpoint, &http_client).await?;
    println!("Logged in successfully!");
    Ok(())
}

async fn generate_hashcash(resource: &str, bits: u32) -> Result<String> {
    let resource = resource.to_owned();

    let stamp = tokio::task::spawn_blocking(move || -> Result<String, anyhow::Error> {
        use chrono::Utc;
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};
        use sha1::{Digest, Sha1};

        let date = Utc::now().format("%y%m%d").to_string();
        let salt: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let mut counter: u64 = 0;

        loop {
            let stamp = format!("1:{}:{}:{}::{}:{:x}", bits, date, resource, salt, counter);
            let mut hasher = Sha1::new();
            hasher.update(stamp.as_bytes());
            let result = hasher.finalize();

            let mut zeros = 0;
            for &byte in result.iter() {
                if byte == 0 {
                    zeros += 8;
                } else {
                    zeros += byte.leading_zeros();
                    break;
                }
            }

            if zeros >= bits {
                return Ok(stamp);
            }
            counter += 1;
        }
    })
    .await??;

    Ok(stamp)
}

#[cfg(test)]
fn verify_hashcash(stamp: &str, expected_bits: u32) -> bool {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(stamp.as_bytes());
    let result = hasher.finalize();

    let mut zeros = 0u32;
    for &byte in result.iter() {
        if byte == 0 {
            zeros += 8;
        } else {
            zeros += byte.leading_zeros();
            break;
        }
    }
    zeros >= expected_bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- inject_token tests ---

    fn make_tools_call(tool_name: &str, arguments: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        })
    }

    #[test]
    fn inject_token_adds_token_to_regular_tool() {
        let mut msg = make_tools_call("get_emails", json!({"limit": 10}));
        inject_token(&mut msg, "test-token-123");

        let token = msg["params"]["arguments"]["token"].as_str().unwrap();
        assert_eq!(token, "test-token-123");
    }

    #[test]
    fn inject_token_preserves_existing_arguments() {
        let mut msg = make_tools_call("get_emails", json!({"limit": 10, "folder": "inbox"}));
        inject_token(&mut msg, "test-token");

        assert_eq!(msg["params"]["arguments"]["limit"], 10);
        assert_eq!(msg["params"]["arguments"]["folder"], "inbox");
        assert_eq!(msg["params"]["arguments"]["token"], "test-token");
    }

    #[test]
    fn inject_token_does_not_overwrite_existing_token() {
        let mut msg = make_tools_call("get_emails", json!({"token": "user-provided-token"}));
        inject_token(&mut msg, "injected-token");

        let token = msg["params"]["arguments"]["token"].as_str().unwrap();
        assert_eq!(token, "user-provided-token");
    }

    #[test]
    fn inject_token_skips_help() {
        let mut msg = make_tools_call("help", json!({}));
        inject_token(&mut msg, "test-token");

        assert!(msg["params"]["arguments"]["token"].is_null());
    }

    #[test]
    fn inject_token_skips_account_create() {
        let mut msg = make_tools_call("account_create", json!({"name": "test", "hashcash": "abc"}));
        inject_token(&mut msg, "test-token");

        assert!(msg["params"]["arguments"]["token"].is_null());
    }

    #[test]
    fn inject_token_skips_auth_exchange() {
        let mut msg = make_tools_call("auth_exchange", json!({"bootstrap_token": "abc"}));
        inject_token(&mut msg, "test-token");

        assert!(msg["params"]["arguments"]["token"].is_null());
    }

    #[test]
    fn inject_token_skips_auth_refresh() {
        let mut msg = make_tools_call("auth_refresh", json!({"refresh_token": "abc"}));
        inject_token(&mut msg, "test-token");

        assert!(msg["params"]["arguments"]["token"].is_null());
    }

    #[test]
    fn inject_token_ignores_non_tools_call_method() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let original = msg.clone();
        inject_token(&mut msg, "test-token");

        assert_eq!(msg, original);
    }

    #[test]
    fn inject_token_ignores_notifications() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let original = msg.clone();
        inject_token(&mut msg, "test-token");

        assert_eq!(msg, original);
    }

    #[test]
    fn inject_token_handles_missing_params() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call"
        });
        let original = msg.clone();
        inject_token(&mut msg, "test-token");

        assert_eq!(msg, original);
    }

    #[test]
    fn inject_token_handles_missing_arguments() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "get_emails"
            }
        });
        inject_token(&mut msg, "test-token");

        // Creates arguments object and injects token
        let args = msg["params"]["arguments"].as_object().unwrap();
        assert_eq!(args["token"], "test-token");
    }

    #[test]
    fn inject_token_handles_missing_name() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "arguments": {"foo": "bar"}
            }
        });
        let original = msg.clone();
        inject_token(&mut msg, "test-token");

        assert_eq!(msg, original);
    }

    #[test]
    fn inject_token_handles_null_name() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": null,
                "arguments": {"foo": "bar"}
            }
        });
        let original = msg.clone();
        inject_token(&mut msg, "test-token");

        assert_eq!(msg, original);
    }

    // --- Credentials tests ---

    #[test]
    fn credentials_serialization_roundtrip() {
        let creds = Credentials {
            access_token: "at-123".to_string(),
            refresh_token: "rt-456".to_string(),
            account_name: "testuser".to_string(),
            endpoint: "https://mcp.inboxapi.ai/mcp".to_string(),
            email: Some("test@example.com".to_string()),
        };

        let json_str = serde_json::to_string(&creds).unwrap();
        let deserialized: Credentials = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.access_token, "at-123");
        assert_eq!(deserialized.refresh_token, "rt-456");
        assert_eq!(deserialized.account_name, "testuser");
        assert_eq!(deserialized.endpoint, "https://mcp.inboxapi.ai/mcp");
    }

    #[test]
    fn credentials_deserialize_from_json() {
        let json_str = r#"{
            "access_token": "abc",
            "refresh_token": "def",
            "account_name": "user1",
            "endpoint": "https://example.com/mcp"
        }"#;

        let creds: Credentials = serde_json::from_str(json_str).unwrap();
        assert_eq!(creds.access_token, "abc");
        assert_eq!(creds.account_name, "user1");
    }

    #[test]
    fn credentials_deserialize_rejects_missing_fields() {
        let json_str = r#"{"access_token": "abc"}"#;
        let result: Result<Credentials, _> = serde_json::from_str(json_str);
        assert!(result.is_err());
    }

    // --- get_credentials_path tests ---

    #[test]
    fn credentials_path_ends_with_expected_components() {
        let path = get_credentials_path().unwrap();
        assert!(path.ends_with("inboxapi/credentials.json"));
    }

    // --- hashcash tests ---

    #[tokio::test]
    async fn generate_hashcash_produces_valid_stamp() {
        let stamp = generate_hashcash("testuser", 8).await.unwrap();

        // Verify format: 1:<bits>:<date>:<resource>::<salt>:<counter>
        let parts: Vec<&str> = stamp.split(':').collect();
        assert_eq!(parts[0], "1");
        assert_eq!(parts[1], "8");
        // parts[2] is date
        assert_eq!(parts[3], "testuser");
        // parts[4] is empty (between :: )
        assert_eq!(parts[4], "");

        // Verify the stamp actually satisfies the proof-of-work
        assert!(verify_hashcash(&stamp, 8));
    }

    #[tokio::test]
    async fn generate_hashcash_contains_resource() {
        let stamp = generate_hashcash("myresource", 8).await.unwrap();
        assert!(stamp.contains("myresource"));
    }

    #[test]
    fn verify_hashcash_rejects_invalid_stamp() {
        assert!(!verify_hashcash("not-a-valid-hashcash", 20));
    }

    // --- agent name generator tests ---

    #[test]
    fn agent_name_has_correct_format() {
        let name = generate_agent_name();
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3, "Expected adj-adj-animal, got: {}", name);
        assert!(!parts[0].is_empty());
        assert!(!parts[1].is_empty());
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn agent_names_produce_variety() {
        let names: std::collections::HashSet<String> =
            (0..50).map(|_| generate_agent_name()).collect();
        assert!(
            names.len() >= 25,
            "Expected at least 25 unique names out of 50, got {}",
            names.len()
        );
    }

    #[test]
    fn agent_name_parts_are_from_word_lists() {
        let all_adjectives: std::collections::HashSet<&str> = [
            "giggly", "wobbly", "bonkers", "goofy", "zany", "wacky", "loopy", "dizzy", "sunny",
            "jolly", "bright", "merry", "chipper", "gleeful", "peppy", "radiant", "fluffy",
            "sparkly", "cozy", "tiny", "snuggly", "precious", "dainty", "fuzzy", "bouncy", "zippy",
            "frisky", "prancy", "bubbly", "perky", "spritely", "jivy",
        ]
        .into_iter()
        .collect();

        let all_animals: std::collections::HashSet<&str> = [
            "penguin",
            "raccoon",
            "owl",
            "cat",
            "capybara",
            "otter",
            "hamster",
            "fox",
            "duckling",
            "panda",
            "ferret",
            "sloth",
            "gecko",
            "hedgehog",
            "bunny",
            "puppy",
            "kitten",
            "dolphin",
            "butterfly",
            "hummingbird",
            "quokka",
            "robin",
            "piglet",
            "lamb",
            "chipmunk",
            "seahorse",
            "koala",
            "honeybee",
            "puffin",
            "fawn",
            "kangaroo",
        ]
        .into_iter()
        .collect();

        for _ in 0..20 {
            let name = generate_agent_name();
            let parts: Vec<&str> = name.split('-').collect();
            assert!(
                all_adjectives.contains(parts[0]),
                "Unknown adjective: {}",
                parts[0]
            );
            assert!(
                all_adjectives.contains(parts[1]),
                "Unknown adjective: {}",
                parts[1]
            );
            assert!(
                all_animals.contains(parts[2]),
                "Unknown animal: {}",
                parts[2]
            );
        }
    }

    // --- is_help_call tests ---

    #[test]
    fn is_help_call_returns_true_for_help_tool() {
        let msg = make_tools_call("help", json!({}));
        assert!(is_help_call(&msg));
    }

    #[test]
    fn is_help_call_returns_false_for_other_tools() {
        let msg = make_tools_call("get_emails", json!({"limit": 10}));
        assert!(!is_help_call(&msg));
    }

    #[test]
    fn is_help_call_returns_false_for_non_tools_call() {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        assert!(!is_help_call(&msg));
    }

    // --- build_help_response tests ---

    #[test]
    fn build_help_response_has_correct_structure() {
        let resp = build_help_response(json!(42));
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 42);
        let content = resp["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("InboxAPI"));
        assert!(text.contains("Authentication is handled automatically"));
    }

    #[test]
    fn build_help_response_preserves_request_id() {
        let resp = build_help_response(json!("req-abc"));
        assert_eq!(resp["id"], "req-abc");
    }

    // --- inject_initialize_instructions tests ---

    #[test]
    fn inject_initialize_instructions_adds_field() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let modified = inject_initialize_instructions(body, None, None);
        let parsed: Value = serde_json::from_str(&modified).unwrap();
        let instructions = parsed["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("handled automatically"));
    }

    #[test]
    fn inject_initialize_instructions_preserves_existing_fields() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"tools":{}},"serverInfo":{"name":"test"}}}"#;
        let modified = inject_initialize_instructions(body, None, None);
        let parsed: Value = serde_json::from_str(&modified).unwrap();
        assert_eq!(parsed["result"]["serverInfo"]["name"], "test");
        assert!(parsed["result"]["capabilities"]["tools"].is_object());
        assert!(parsed["result"]["instructions"].is_string());
    }

    #[test]
    fn inject_initialize_instructions_returns_unchanged_on_invalid_json() {
        let body = "not valid json";
        let result = inject_initialize_instructions(body, None, None);
        assert_eq!(result, "not valid json");
    }

    #[test]
    fn inject_initialize_instructions_with_creds_includes_identity() {
        let creds = Credentials {
            access_token: "at".to_string(),
            refresh_token: "rt".to_string(),
            account_name: "test-agent".to_string(),
            endpoint: "https://example.com".to_string(),
            email: Some("test-agent@inboxapi.io".to_string()),
        };
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let modified = inject_initialize_instructions(body, Some(&creds), None);
        let parsed: Value = serde_json::from_str(&modified).unwrap();
        let instructions = parsed["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("test-agent"));
        assert!(instructions.contains("test-agent@inboxapi.io"));
        assert!(instructions.contains("from_name"));
    }

    // --- rewrite_tools_list tests ---

    fn make_tools_list_response(tools: Vec<Value>) -> String {
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": tools
            }
        }))
        .unwrap()
    }

    #[test]
    fn rewrite_tools_list_rewrites_auth_tools() {
        let body = make_tools_list_response(vec![
            json!({"name": "account_create", "description": "Step 1: Check ~/.local/inboxapi/credentials.json first..."}),
            json!({"name": "auth_exchange", "description": "Step 2: Exchange your bootstrap token..."}),
            json!({"name": "auth_refresh", "description": "Step 3 (when needed): Refresh an expired access token..."}),
        ]);
        let result = rewrite_tools_list(&body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();

        // Auth tools should be rewritten; injected tools are separate
        for tool in tools.iter().filter(|t| {
            !["whoami", "report_bug", "request_feature"].contains(&t["name"].as_str().unwrap_or(""))
        }) {
            assert_eq!(tool["description"], AUTH_TOOL_OVERRIDE);
        }
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"whoami"));
    }

    #[test]
    fn rewrite_tools_list_preserves_other_tools() {
        let body = make_tools_list_response(vec![
            json!({"name": "get_emails", "description": "Fetch emails from your inbox"}),
            json!({"name": "help", "description": "Show help text"}),
            json!({"name": "auth_introspect", "description": "Check token status"}),
            json!({"name": "account_create", "description": "Old description"}),
        ]);
        let result = rewrite_tools_list(&body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();

        assert_eq!(tools[0]["description"], "Fetch emails from your inbox");
        assert_eq!(tools[1]["description"], "Show help text");
        assert_eq!(tools[2]["description"], AUTH_TOOL_OVERRIDE);
        assert_eq!(tools[3]["description"], AUTH_TOOL_OVERRIDE);
    }

    #[test]
    fn rewrite_tools_list_preserves_tool_fields() {
        let body = make_tools_list_response(vec![json!({
            "name": "account_create",
            "description": "Old description",
            "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}}
        })]);
        let result = rewrite_tools_list(&body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tool = &parsed["result"]["tools"][0];

        assert_eq!(tool["description"], AUTH_TOOL_OVERRIDE);
        assert_eq!(tool["inputSchema"]["type"], "object");
    }

    #[test]
    fn rewrite_tools_list_handles_no_tools() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let result = rewrite_tools_list(body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        // Only the injected local tools should be present
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"whoami"));
        assert!(names.contains(&"report_bug"));
        assert!(names.contains(&"request_feature"));
    }

    #[test]
    fn rewrite_tools_list_returns_unchanged_on_invalid_json() {
        let body = "not valid json";
        assert_eq!(rewrite_tools_list(body, None), "not valid json");
    }

    #[test]
    fn rewrite_tools_list_returns_unchanged_without_result() {
        let body = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"fail"}}"#;
        let result = rewrite_tools_list(body, None);
        assert_eq!(result, body);
    }

    #[test]
    fn inject_initialize_instructions_returns_unchanged_without_result() {
        let body = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"fail"}}"#;
        let result = inject_initialize_instructions(body, None, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["result"]["instructions"].is_null());
    }

    fn make_test_creds() -> Credentials {
        Credentials {
            access_token: "at".to_string(),
            refresh_token: "rt".to_string(),
            account_name: "cool-agent".to_string(),
            endpoint: "https://example.com".to_string(),
            email: Some("cool-agent@inboxapi.io".to_string()),
        }
    }

    #[test]
    fn rewrite_tools_list_with_creds_annotates_identity_tools() {
        let creds = make_test_creds();
        let body = make_tools_list_response(vec![
            json!({"name": "send_email", "description": "Send an email."}),
            json!({"name": "send_reply", "description": "Reply to an email."}),
            json!({"name": "forward_email", "description": "Forward an email."}),
            json!({"name": "get_emails", "description": "Get emails."}),
        ]);
        let result = rewrite_tools_list(&body, Some(&creds));
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();

        // send_email, send_reply, forward_email should contain identity
        for i in 0..3 {
            let desc = tools[i]["description"].as_str().unwrap();
            assert!(
                desc.contains("cool-agent"),
                "tool {} missing name",
                tools[i]["name"]
            );
            assert!(
                desc.contains("cool-agent@inboxapi.io"),
                "tool {} missing email",
                tools[i]["name"]
            );
            assert!(
                desc.contains("Cool Agent"),
                "tool {} missing display name",
                tools[i]["name"]
            );
            assert!(
                desc.contains("do not sign as the AI model"),
                "tool {} missing signing guidance",
                tools[i]["name"]
            );
        }
        // get_emails should NOT be annotated
        let get_desc = tools[3]["description"].as_str().unwrap();
        assert!(!get_desc.contains("cool-agent"));
    }

    #[test]
    fn rewrite_tools_list_whoami_includes_identity_when_creds_present() {
        let creds = make_test_creds();
        let body = make_tools_list_response(vec![]);
        let result = rewrite_tools_list(&body, Some(&creds));
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        let whoami = tools
            .iter()
            .find(|t| t["name"].as_str() == Some("whoami"))
            .unwrap();
        let desc = whoami["description"].as_str().unwrap();
        assert!(desc.contains("cool-agent"));
        assert!(desc.contains("cool-agent@inboxapi.io"));
    }

    #[test]
    fn sanitize_for_description_strips_dangerous_chars() {
        assert_eq!(sanitize_for_description("good-name_123"), "good-name_123");
        assert_eq!(sanitize_for_description("user@domain.io"), "user@domain.io");
        assert_eq!(
            sanitize_for_description("evil<script>alert('xss')</script>"),
            "evilscriptalertxssscript"
        );
        assert_eq!(
            sanitize_for_description("name with spaces"),
            "namewithspaces"
        );
    }

    #[test]
    fn sanitize_for_description_truncates_long_input() {
        let long_input = "a".repeat(200);
        let result = sanitize_for_description(&long_input);
        assert_eq!(result.len(), 128);
    }

    #[test]
    fn display_name_from_account_converts_hyphenated_names() {
        assert_eq!(display_name_from_account("cool-agent"), "Cool Agent");
        assert_eq!(
            display_name_from_account("brooding-sinister-cat"),
            "Brooding Sinister Cat"
        );
        assert_eq!(display_name_from_account("agent"), "Agent");
    }

    // --- creds without email edge case tests ---

    fn make_creds_without_email() -> Credentials {
        Credentials {
            access_token: "at".to_string(),
            refresh_token: "rt".to_string(),
            account_name: "no-email-agent".to_string(),
            endpoint: "https://example.com".to_string(),
            email: None,
        }
    }

    #[test]
    fn inject_initialize_instructions_with_creds_no_email_skips_identity() {
        let creds = make_creds_without_email();
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let modified = inject_initialize_instructions(body, Some(&creds), None);
        let parsed: Value = serde_json::from_str(&modified).unwrap();
        let instructions = parsed["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("handled automatically"));
        assert!(!instructions.contains("no-email-agent"));
    }

    #[test]
    fn rewrite_tools_list_with_creds_no_email_skips_identity_tools() {
        let creds = make_creds_without_email();
        let body = make_tools_list_response(vec![
            json!({"name": "send_email", "description": "Send an email."}),
            json!({"name": "get_emails", "description": "Get emails."}),
        ]);
        let result = rewrite_tools_list(&body, Some(&creds));
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();

        // send_email should NOT be annotated when email is missing
        assert_eq!(tools[0]["description"], "Send an email.");
        // whoami should use the default description
        let whoami = tools
            .iter()
            .find(|t| t["name"].as_str() == Some("whoami"))
            .unwrap();
        let desc = whoami["description"].as_str().unwrap();
        assert!(!desc.contains("no-email-agent"));
        assert!(desc.contains("Returns this agent's own identity:"));
    }

    // --- is_token_expired_error tests ---

    fn make_error_response(text: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "isError": true,
                "content": [{"type": "text", "text": text}]
            }
        })
    }

    #[test]
    fn is_token_expired_error_detects_expired_token() {
        let resp = make_error_response("Token is invalid, expired, or revoked");
        assert!(is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_detects_invalid_token() {
        let resp = make_error_response("Invalid token provided");
        assert!(is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_detects_revoked_token() {
        let resp = make_error_response("This token has been revoked");
        assert!(is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_case_insensitive() {
        let resp = make_error_response("TOKEN IS EXPIRED");
        assert!(is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_rejects_success_response() {
        let resp = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{"type": "text", "text": "here are your emails"}]
            }
        });
        assert!(!is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_rejects_non_token_errors() {
        let resp = make_error_response("Rate limit exceeded");
        assert!(!is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_handles_missing_is_error() {
        let resp = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{"type": "text", "text": "Token expired"}]
            }
        });
        assert!(!is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_handles_empty_content() {
        let resp = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "isError": true,
                "content": []
            }
        });
        assert!(!is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_detects_jsonrpc_auth_error() {
        let resp = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32001,
                "message": "Authentication failed: Invalid or expired token"
            }
        });
        assert!(is_token_expired_error(&resp));
    }

    #[test]
    fn is_token_expired_error_ignores_non_token_jsonrpc_error() {
        let resp = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32600,
                "message": "Invalid request"
            }
        });
        assert!(!is_token_expired_error(&resp));
    }

    // --- mutate_feedback_tool tests ---

    #[test]
    fn mutate_feedback_tool_rewrites_report_bug() {
        let creds = make_test_creds();
        let mut msg = make_tools_call(
            "report_bug",
            json!({"subject": "Login fails", "body": "Steps to reproduce..."}),
        );
        assert!(mutate_feedback_tool(&mut msg, Some(&creds)));
        assert_eq!(msg["params"]["name"], "send_email");
        assert_eq!(msg["params"]["arguments"]["to"][0], "bugs@inboxapi.dev");
        assert_eq!(
            msg["params"]["arguments"]["subject"],
            "[Bug Report] Login fails"
        );
        assert_eq!(msg["params"]["arguments"]["body"], "Steps to reproduce...");
        assert_eq!(msg["params"]["arguments"]["from_name"], "cool-agent");
    }

    #[test]
    fn mutate_feedback_tool_rewrites_request_feature() {
        let creds = make_test_creds();
        let mut msg = make_tools_call(
            "request_feature",
            json!({"subject": "Add labels", "body": "Would be nice..."}),
        );
        assert!(mutate_feedback_tool(&mut msg, Some(&creds)));
        assert_eq!(msg["params"]["name"], "send_email");
        assert_eq!(msg["params"]["arguments"]["to"][0], "features@inboxapi.dev");
        assert_eq!(
            msg["params"]["arguments"]["subject"],
            "[Feature Request] Add labels"
        );
    }

    #[test]
    fn mutate_feedback_tool_ignores_other_tools() {
        let mut msg = make_tools_call("get_emails", json!({"limit": 10}));
        let original = msg.clone();
        assert!(!mutate_feedback_tool(&mut msg, None));
        assert_eq!(msg, original);
    }

    #[test]
    fn mutate_feedback_tool_ignores_non_tools_call() {
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let original = msg.clone();
        assert!(!mutate_feedback_tool(&mut msg, None));
        assert_eq!(msg, original);
    }

    #[test]
    fn mutate_feedback_tool_handles_missing_args() {
        let mut msg = make_tools_call("report_bug", json!({}));
        assert!(mutate_feedback_tool(&mut msg, None));
        assert_eq!(
            msg["params"]["arguments"]["subject"],
            "[Bug Report] (no subject)"
        );
        assert_eq!(msg["params"]["arguments"]["body"], "(no body)");
    }

    #[test]
    fn mutate_feedback_tool_without_creds() {
        let mut msg = make_tools_call("report_bug", json!({"subject": "Bug", "body": "Details"}));
        assert!(mutate_feedback_tool(&mut msg, None));
        assert_eq!(msg["params"]["name"], "send_email");
        assert!(msg["params"]["arguments"]["from_name"].is_null());
    }

    #[test]
    fn mutate_feedback_tool_no_stale_token() {
        let creds = make_test_creds();
        let mut msg = make_tools_call(
            "report_bug",
            json!({"subject": "Bug", "body": "Details", "token": "old-token"}),
        );
        assert!(mutate_feedback_tool(&mut msg, Some(&creds)));
        // Old arguments (including token) should not carry over
        assert!(msg["params"]["arguments"]["token"].is_null());
    }

    // --- rewrite_tools_list feedback tool tests ---

    #[test]
    fn rewrite_tools_list_injects_feedback_tools() {
        let body = make_tools_list_response(vec![]);
        let result = rewrite_tools_list(&body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"report_bug"));
        assert!(names.contains(&"request_feature"));
    }

    #[test]
    fn rewrite_tools_list_feedback_tool_schemas() {
        let body = make_tools_list_response(vec![]);
        let result = rewrite_tools_list(&body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();

        for name in &["report_bug", "request_feature"] {
            let tool = tools
                .iter()
                .find(|t| t["name"].as_str() == Some(name))
                .unwrap();
            let schema = &tool["inputSchema"];
            assert_eq!(schema["type"], "object");
            let props = schema["properties"].as_object().unwrap();
            assert!(props.contains_key("subject"));
            assert!(props.contains_key("body"));
            assert_eq!(props["subject"]["type"], "string");
            assert_eq!(props["body"]["type"], "string");
            let required = schema["required"].as_array().unwrap();
            assert!(required.contains(&json!("subject")));
            assert!(required.contains(&json!("body")));
        }
    }

    #[test]
    fn rewrite_tools_list_feedback_descriptions_scoped() {
        let body = make_tools_list_response(vec![]);
        let result = rewrite_tools_list(&body, None);
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();

        for name in &["report_bug", "request_feature"] {
            let tool = tools
                .iter()
                .find(|t| t["name"].as_str() == Some(name))
                .unwrap();
            let desc = tool["description"].as_str().unwrap();
            assert!(
                desc.contains("InboxAPI"),
                "{} description should mention InboxAPI",
                name
            );
        }
    }

    // --- version comparison tests ---

    #[test]
    fn compare_versions_equal() {
        assert_eq!(compare_versions("1.2.3", "1.2.3"), Ordering::Equal);
    }

    #[test]
    fn compare_versions_newer() {
        assert_eq!(compare_versions("1.2.4", "1.2.3"), Ordering::Greater);
    }

    #[test]
    fn compare_versions_older() {
        assert_eq!(compare_versions("1.2.3", "1.2.4"), Ordering::Less);
    }

    #[test]
    fn compare_versions_major_diff() {
        assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
    }

    #[test]
    fn is_newer_true() {
        assert!(is_newer("1.1.0", "1.0.0"));
    }

    #[test]
    fn is_newer_false_equal() {
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn is_newer_false_older() {
        assert!(!is_newer("0.9.0", "1.0.0"));
    }

    // --- inject_update_notice tests ---

    #[test]
    fn inject_update_notice_appends_content() {
        let mut response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{"type": "text", "text": "original"}]
            }
        });
        inject_update_notice(&mut response, "2.0.0");
        let content = response["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        let notice = content[1]["text"].as_str().unwrap();
        assert!(notice.contains("2.0.0"));
        assert!(notice.contains("npm install"));
    }

    #[test]
    fn inject_update_notice_preserves_existing() {
        let mut response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {"type": "text", "text": "first"},
                    {"type": "text", "text": "second"}
                ]
            }
        });
        inject_update_notice(&mut response, "3.0.0");
        let content = response["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 3);
        assert_eq!(content[0]["text"], "first");
        assert_eq!(content[1]["text"], "second");
        assert!(content[2]["text"].as_str().unwrap().contains("3.0.0"));
    }

    #[test]
    fn inject_update_notice_handles_missing_content() {
        let mut response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });
        inject_update_notice(&mut response, "2.0.0");
        // Should not crash; no content array to append to
        assert!(response["result"]["content"].is_null());
    }

    // --- inject_initialize_instructions with update ---

    #[test]
    fn inject_initialize_instructions_with_update() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let modified = inject_initialize_instructions(body, None, Some("2.0.0"));
        let parsed: Value = serde_json::from_str(&modified).unwrap();
        let instructions = parsed["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("outdated"));
        assert!(instructions.contains("2.0.0"));
        assert!(instructions.contains("npm install"));
    }

    #[test]
    fn inject_initialize_instructions_without_update() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let modified = inject_initialize_instructions(body, None, None);
        let parsed: Value = serde_json::from_str(&modified).unwrap();
        let instructions = parsed["result"]["instructions"].as_str().unwrap();
        assert!(!instructions.contains("outdated"));
    }

    // --- version cache tests ---

    #[test]
    fn is_cache_stale_old_cache() {
        let cache = VersionCache {
            latest_version: "1.0.0".to_string(),
            checked_at: "2020-01-01T00:00:00+00:00".to_string(),
        };
        assert!(is_cache_stale(&cache));
    }

    #[test]
    fn is_cache_stale_fresh_cache() {
        let cache = VersionCache {
            latest_version: "1.0.0".to_string(),
            checked_at: chrono::Utc::now().to_rfc3339(),
        };
        assert!(!is_cache_stale(&cache));
    }

    #[test]
    fn is_cache_stale_invalid_date() {
        let cache = VersionCache {
            latest_version: "1.0.0".to_string(),
            checked_at: "not-a-date".to_string(),
        };
        assert!(is_cache_stale(&cache));
    }

    #[test]
    fn is_cache_stale_future_timestamp() {
        let cache = VersionCache {
            latest_version: "1.0.0".to_string(),
            checked_at: "2099-01-01T00:00:00+00:00".to_string(),
        };
        assert!(is_cache_stale(&cache));
    }

    #[test]
    fn version_cache_roundtrip() {
        let cache = VersionCache {
            latest_version: "1.2.3".to_string(),
            checked_at: chrono::Utc::now().to_rfc3339(),
        };
        let json_str = serde_json::to_string(&cache).unwrap();
        let parsed: VersionCache = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.latest_version, "1.2.3");
    }

    // --- SSE parser tests ---

    #[test]
    fn drain_sse_events_parses_single_event() {
        let mut buf = "event: message\ndata: {\"id\":1}\n\n".to_string();
        let events = drain_sse_events(&mut buf);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{\"id\":1}");
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_sse_events_parses_multiple_events() {
        let mut buf = "data: first\n\ndata: second\n\n".to_string();
        let events = drain_sse_events(&mut buf);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_sse_events_preserves_incomplete_buffer() {
        let mut buf = "data: complete\n\ndata: incomp".to_string();
        let events = drain_sse_events(&mut buf);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "complete");
        assert_eq!(buf, "data: incomp");
    }

    #[test]
    fn drain_sse_events_filters_non_message_events() {
        let mut buf = "event: ping\ndata: {}\n\nevent: message\ndata: hello\n\n".to_string();
        let events = drain_sse_events(&mut buf);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn drain_sse_events_handles_multiline_data() {
        let mut buf = "data: line1\ndata: line2\ndata: line3\n\n".to_string();
        let events = drain_sse_events(&mut buf);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2\nline3");
    }

    #[test]
    fn drain_sse_events_returns_empty_for_no_complete_events() {
        let mut buf = "data: partial".to_string();
        let events = drain_sse_events(&mut buf);
        assert!(events.is_empty());
        assert_eq!(buf, "data: partial");
    }

    #[test]
    fn drain_sse_events_returns_empty_for_empty_input() {
        let mut buf = String::new();
        let events = drain_sse_events(&mut buf);
        assert!(events.is_empty());
    }

    #[test]
    fn drain_sse_events_treats_missing_event_type_as_message() {
        let mut buf = "data: implicit_message\n\n".to_string();
        let events = drain_sse_events(&mut buf);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "implicit_message");
    }

    #[test]
    fn drain_sse_remainder_parses_trailing_event() {
        let buf = "event: message\ndata: trailing";
        let event = drain_sse_remainder(buf);
        assert!(event.is_some());
        assert_eq!(event.unwrap().data, "trailing");
    }

    #[test]
    fn drain_sse_remainder_returns_none_for_empty_buffer() {
        assert!(drain_sse_remainder("").is_none());
        assert!(drain_sse_remainder("  \n  ").is_none());
    }

    #[test]
    fn drain_sse_remainder_filters_non_message_events() {
        let buf = "event: ping\ndata: {}";
        assert!(drain_sse_remainder(buf).is_none());
    }
}
