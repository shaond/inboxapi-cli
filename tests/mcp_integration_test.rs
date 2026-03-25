use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use serde_json::{json, Value};

struct McpTestClient {
    child: Child,
}

impl McpTestClient {
    fn spawn() -> Self {
        let child = Command::new("cargo")
            .args(["run", "--", "proxy"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn cargo run -- proxy");
        Self { child }
    }

    fn send(&mut self, request: Value) {
        let stdin = self.child.stdin.as_mut().expect("Failed to open stdin");
        let line = serde_json::to_string(&request).expect("Failed to serialize request");
        writeln!(stdin, "{}", line).expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    fn receive(&mut self) -> Value {
        let stdout = self.child.stdout.as_mut().expect("Failed to open stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).expect("Failed to read from stdout");
        serde_json::from_str(&line).expect("Failed to deserialize response")
    }

    fn kill(mut self) {
        let _ = self.child.kill();
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
    assert_eq!(response["id"], 1);
    assert!(response["result"]["protocolVersion"].is_string());
    assert!(response["result"]["capabilities"]["tools"].is_object());

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
    assert_eq!(response["id"], 2);
    let tools = response["result"]["tools"].as_array().expect("Tools should be an array");
    assert!(!tools.is_empty(), "Tool list should not be empty");

    // Check for core tools
    let tool_names: Vec<&str> = tools.iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    
    println!("Actual tool names: {:?}", tool_names);
    
    assert!(tool_names.contains(&"get_emails"), "Missing get_emails in {:?}", tool_names);
    assert!(tool_names.contains(&"send_email"), "Missing send_email in {:?}", tool_names);

    client.kill();
}
