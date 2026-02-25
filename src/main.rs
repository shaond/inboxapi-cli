use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use eventsource_client::{Client, SSE};
use futures_util::StreamExt;
use reqwest::{Client as HttpClient, header::{CONTENT_TYPE, ACCEPT}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader, stdin, stdout, AsyncWriteExt};

#[derive(Parser)]
#[command(name = "inboxapi")]
#[command(about = "InboxAPI CLI - STDIO Proxy for InboxAPI MCP service", long_about = None)]
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Credentials {
    access_token: String,
    refresh_token: String,
    account_name: String,
    endpoint: String,
}

fn get_credentials_path() -> Result<PathBuf> {
    let base_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not determine configuration directory"))?;
    Ok(base_dir.join("inboxapi").join("credentials.json"))
}

fn load_credentials() -> Result<Credentials> {
    let path = get_credentials_path()?;
    let content = std::fs::read_to_string(path).context("Could not read credentials file. Have you run 'inboxapi login'?")?;
    serde_json::from_str(&content).context("Failed to parse credentials file")
}

fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = get_credentials_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(creds)?;

    #[cfg(unix)]
    {
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Login { name, endpoint }) => login_flow(name, endpoint).await,
        Some(Commands::Whoami) => {
            let creds = load_credentials()?;
            println!("Logged in as: {}", creds.account_name);
            println!("Endpoint: {}", creds.endpoint);
            Ok(())
        }
        Some(Commands::Proxy { endpoint }) => run_proxy(endpoint).await,
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

async fn run_proxy(endpoint: String) -> Result<()> {
    // 1. Connect to SSE
    let client = eventsource_client::ClientBuilder::for_url(&endpoint)?
        .header(ACCEPT.as_str(), "text/event-stream")?
        .build();

    let mut sse_stream = client.stream();
    let http_client = HttpClient::new();
    let creds = load_credentials().ok();

    // Spawn task for handling SSE -> stdout
    tokio::spawn(async move {
        let mut out = stdout();
        while let Some(event) = sse_stream.next().await {
            match event {
                Ok(SSE::Event(ev)) => {
                    // In MCP Streamable HTTP, messages might come as "message" events or similar
                    // The standard SSE transport for MCP sends JSON-RPC in the "message" event data
                    if ev.event_type == "message" {
                        if let Err(e) = out.write_all(format!("{}\n", ev.data).as_bytes()).await {
                            eprintln!("Failed to write to stdout: {}", e);
                            break;
                        }
                        if let Err(e) = out.flush().await {
                            eprintln!("Failed to flush stdout: {}", e);
                            break;
                        }
                    } else if ev.event_type == "endpoint" {
                        // The server might send a new endpoint to POST to, but in our case it's usually the same
                        // If it's different, we should probably update it.
                        // However, for simplicity let's just ignore for now if it matches.
                    }
                }
                Ok(SSE::Comment(_)) => {}
                Err(e) => {
                    eprintln!("SSE Error: {}", e);
                }
            }
        }
    });

    // Handle stdin -> POST
    let mut lines = BufReader::new(stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        let mut msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Inject token if needed
        if let Some(creds) = &creds {
            inject_token(&mut msg, &creds.access_token);
        }

        // Post to endpoint
        let res = http_client.post(&endpoint)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/event-stream")
            .json(&msg)
            .send()
            .await;

        match res {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let err_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    eprintln!("POST failed ({}): {}", status, err_text);
                }
            }
            Err(e) => {
                eprintln!("POST Error: {}", e);
            }
        }
    }

    Ok(())
}

fn inject_token(msg: &mut Value, token: &str) {
    if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        // Only inject for tool calls
        if method == "tools/call" {
            if let Some(params) = msg.get_mut("params").and_then(|p| p.as_object_mut()) {
                if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                    // Skip public/auth tools that don't need or use different tokens
                    if name == "help" || name == "account_create" || name == "auth_exchange" || name == "auth_refresh" {
                        return;
                    }

                    if let Some(arguments) = params.get_mut("arguments").and_then(|a| a.as_object_mut()) {
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

    println!("Generating hashcash for: {}...", name);
    let hashcash = generate_hashcash(&name, 20).await?;
    println!("Hashcash generated: {}", hashcash);

    let http_client = HttpClient::new();

    // 1. account_create
    println!("Creating account...");
    let resp = http_client.post(&endpoint)
        .header(CONTENT_TYPE, "application/json")
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
        .await?
        .json::<Value>()
        .await?;

    // Parse the response from tools/call
    let content = resp.get("result").and_then(|r| r.get("content")).and_then(|c| c.as_array())
        .and_then(|c| c.get(0)).and_then(|c| c.get("text")).and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Failed to parse account_create response: {:?}", resp))?;

    let account_data: Value = serde_json::from_str(content)?;
    let bootstrap_token = account_data["bootstrap_token"].as_str()
        .ok_or_else(|| anyhow!("Missing bootstrap_token in response"))?;

    // 2. auth_exchange
    println!("Exchanging bootstrap token for access tokens...");
    let resp = http_client.post(&endpoint)
        .header(CONTENT_TYPE, "application/json")
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
        .await?
        .json::<Value>()
        .await?;

    let content = resp.get("result").and_then(|r| r.get("content")).and_then(|c| c.as_array())
        .and_then(|c| c.get(0)).and_then(|c| c.get("text")).and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Failed to parse auth_exchange response: {:?}", resp))?;

    let token_data: Value = serde_json::from_str(content)?;
    let access_token = token_data["access_token"].as_str().ok_or_else(|| anyhow!("Missing access_token"))?;
    let refresh_token = token_data["refresh_token"].as_str().ok_or_else(|| anyhow!("Missing refresh_token"))?;

    let creds = Credentials {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        account_name: name,
        endpoint: endpoint.clone(),
    };

    save_credentials(&creds)?;
    println!("Logged in successfully!");
    Ok(())
}

async fn generate_hashcash(resource: &str, bits: u32) -> Result<String> {
    let resource = resource.to_owned();

    let stamp = tokio::task::spawn_blocking(move || -> Result<String, anyhow::Error> {
        use sha1::{Sha1, Digest};
        use chrono::Utc;
        use rand::{thread_rng, Rng};
        use rand::distributions::Alphanumeric;

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
