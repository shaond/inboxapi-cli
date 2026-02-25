# InboxAPI CLI - Gemini Agent Context

## Repository Overview
InboxAPI CLI is a Rust-based STDIO proxy for the InboxAPI MCP service. It provides a simple, cross-platform bridge for AI tools to access email capabilities through a remote MCP endpoint.

## Technology Stack
- **Language:** Rust (Tokio, reqwest, eventsource-client)
- **Protocol:** JSON-RPC (MCP standard over STDIO)
- **Transport:** HTTP/SSE (to remote service)
- **Packaging:** Planned as npm package (`@inboxapi/cli`)

## Environment & Setup
- **Credentials:** Stored under the OS config directory (e.g. `~/.config/inboxapi/credentials.json` on Linux).
- **Remote Endpoint:** Default is `https://mcp.inboxapi.ai/mcp`.

## Development Commands
```bash
cargo check     # Type check
cargo build     # Build binary
cargo run -- login # Authentication flow
cargo run -- proxy # Start proxy (default)
```

## Implementation Notes
- **Authentication:** Injects the access token value as the `token` field in the `arguments` of `tools/call` JSON-RPC messages when the method is `tools/call` and the `token` argument is missing.
- **Hashcash:** Custom SHA-1 implementation in `src/main.rs` for `account_create`.
- **Proxy Loop:**
  - `stdin` -> `POST` to remote.
  - `SSE` from remote -> `stdout`.

## Release Strategy
For npm distribution, we recommend a main package `@inboxapi/cli` that uses platform-specific optional dependencies containing the pre-compiled Rust binaries. This is the standard approach used by tools like `esbuild` and `swc`.

## Synchronizing with `inboxapi-mcp`
Since this CLI is a pure proxy, it only needs to be updated if:
1. The transport protocol (SSE/Streamable HTTP) changes.
2. The authentication method (token injection logic) changes.
3. The remote endpoint address changes.

The specific tools and resources are handled opaquely by the proxy, so changes to individual tools in `inboxapi-mcp` do not require updates to this CLI.
