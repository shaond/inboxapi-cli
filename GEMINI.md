# InboxAPI CLI — Gemini Agent Context

## Overview
Rust STDIO proxy that bridges JSON-RPC (MCP protocol) over STDIO to the remote InboxAPI MCP service over Streamable HTTP/SSE. Distributed as `@inboxapi/cli` on npm with platform-specific binary packages.

## Technology Stack
- **Language:** Rust (Tokio async runtime, reqwest HTTP client, eventsource-client SSE)
- **Protocol:** JSON-RPC over STDIO (MCP standard)
- **Transport:** HTTP POST + SSE to remote endpoint
- **Distribution:** npm with platform-specific optional dependencies
- **CI:** GitHub Actions (5-platform cross-build, GitHub Releases, npm publish)

## Commands
```bash
cargo build          # Build binary
cargo test           # Run tests
cargo fmt            # Format code
cargo run -- proxy   # Start STDIO proxy (default)
cargo run -- login   # Authenticate and store credentials
cargo run -- whoami  # Show current account
```

## Development Workflow
- Single Rust source file (`src/main.rs`) containing all proxy, auth, and CLI logic
- `index.js` is the npm entry point — resolves platform binary or falls back to `cargo run`
- Tags (`v*.*.*`) trigger CI builds and npm publishing

## Key Files
- `src/main.rs` — Proxy loop, token injection, login flow, hashcash proof-of-work
- `index.js` — npm binary resolver (platform package → local build → cargo run)
- `package.json` — Root npm package with `optionalDependencies` for 5 platforms
- `npm/cli-*/package.json` — Platform-specific package manifests (os/cpu fields)
- `.github/workflows/release.yml` — Cross-build, GitHub Release, npm publish pipeline
- `Cargo.toml` — Rust project configuration and dependencies

## npm Distribution
```
@inboxapi/cli              — Main package (wrapper script)
@inboxapi/cli-darwin-arm64 — macOS ARM64
@inboxapi/cli-darwin-x64   — macOS x64
@inboxapi/cli-linux-x64    — Linux x64
@inboxapi/cli-linux-arm64  — Linux ARM64
@inboxapi/cli-win32-x64    — Windows x64
```

Users install with `npm install -g @inboxapi/cli`. npm automatically selects the correct platform binary via the `os` and `cpu` fields in each platform package.

## Rules
- Do not add AI attribution to commits, code, or comments
- Run `cargo fmt` before committing Rust changes
- Use conventional commits (`feat:`, `fix:`, `chore:`, etc.)
