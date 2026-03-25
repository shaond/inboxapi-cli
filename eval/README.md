# InboxAPI Agent Evaluation Script

This script evaluates how well an LLM (Claude) understands and uses the InboxAPI CLI tools via the MCP protocol.

## Prerequisites

1.  **Anthropic API Key:** Set the `ANTHROPIC_API_KEY` environment variable.
2.  **InboxAPI Authentication:** Ensure you are logged in (`cargo run -- login`).
3.  **Recent Contacts:** You must have at least one contact in your addressbook (`cargo run -- get-addressbook`).
4.  **Node.js:** Installed and available.

## Setup

```bash
cd eval
npm install
```

## Running Evaluations

```bash
node index.js
```

The script will:
1.  Verify your InboxAPI authentication.
2.  Retrieve your recent contacts and ask you to select one as a test recipient.
3.  Provide Claude with your current MCP tool definitions.
4.  Run a series of natural language prompts ("intents") and verify that Claude generates the correct tool calls.
5.  Execute those tool calls against the local CLI proxy to verify end-to-end connectivity.
