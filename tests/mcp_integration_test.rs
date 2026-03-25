use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const READ_TIMEOUT: Duration = Duration::from_secs(30);

struct McpTestClient {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl McpTestClient {
    fn spawn() -> Self {
        let mut child = Command::new("cargo")
            .args(["run", "--", "proxy"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn cargo run -- proxy");
        let stdout = child.stdout.take().expect("Failed to take stdout");
        let reader = BufReader::new(stdout);
        Self { child, reader }
    }

    fn send(&mut self, request: Value) {
        let stdin = self.child.stdin.as_mut().expect("Failed to open stdin");
        let line = serde_json::to_string(&request).expect("Failed to serialize request");
        writeln!(stdin, "{}", line).expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    fn receive(&mut self) -> Value {
        let start = Instant::now();
        loop {
            let mut line = String::new();
            let bytes = self
                .reader
                .read_line(&mut line)
                .expect("Failed to read from stdout");
            if bytes == 0 {
                if start.elapsed() > READ_TIMEOUT {
                    panic!("Timeout after {:?} waiting for MCP response", READ_TIMEOUT);
                }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            return serde_json::from_str(trimmed).unwrap_or_else(|e| {
                panic!("Failed to deserialize response: {e}\nLine: {trimmed}")
            });
        }
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn test_mcp_handshake() {
    let mut client = McpTestClient::spawn();

    // 1. Initialize
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    }));

    let response = client.receive();
    assert_eq!(response["id"], 1, "Expected initialize response id=1");
    assert!(
        response["result"]["protocolVersion"].is_string(),
        "protocolVersion should be a string, got: {}",
        response
    );
    assert!(
        response["result"]["capabilities"]["tools"].is_object(),
        "capabilities.tools should be an object, got: {}",
        response
    );

    // 2. Initialized notification
    client.send(json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }));

    // 3. List tools
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    }));

    let response = client.receive();
    assert_eq!(response["id"], 2, "Expected tools/list response id=2");
    let tools = response["result"]["tools"]
        .as_array()
        .expect("Tools should be an array");
    assert!(!tools.is_empty(), "Tool list should not be empty");

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(
        tool_names.contains(&"get_emails"),
        "Missing get_emails in {:?}",
        tool_names
    );
    assert!(
        tool_names.contains(&"send_email"),
        "Missing send_email in {:?}",
        tool_names
    );
}
