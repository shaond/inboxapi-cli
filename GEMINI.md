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

# CLI subcommands
cargo run -- send-email --to user@example.com --subject "Hi" --body "Hello"
cargo run -- get-emails --limit 5
cargo run -- get-email "<message-id>"
cargo run -- search-emails --subject "keyword"
cargo run -- get-attachment <id> --output ./file.pdf
cargo run -- send-reply --message-id "<id>" --body "Reply"
cargo run -- forward-email --message-id "<id>" --to user@example.com
cargo run -- help
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

## Coding Standards

### Rust
- Implement `Drop` for structs owning child processes or OS resources — panics must not leak processes
- Store `BufReader` in structs rather than re-creating per call — re-creation loses buffered data
- Prefer iterators (`iter().take(n)`) over index loops (`for i in 0..n`) when indexing a single collection
- Add timeouts to all blocking I/O — tests and tools must not hang indefinitely
- Include descriptive messages in all assertions

### JavaScript / Node.js
- Pass arguments as arrays to `execFileSync` — never use string interpolation with `execSync` (shell injection risk)
- Do not declare synchronous functions as `async`
- Use line-based parsing (`readline.createInterface`) for child process stdout — raw `data` events are chunked arbitrarily
- Spawn subprocesses once and reuse — do not spawn a new process per request
- Validate user input before using it to index arrays or build commands
- Centralize model identifiers in a single constant or environment variable; avoid scattering hardcoded dated model version strings throughout the code

### MCP Protocol
- Always send `notifications/initialized` after `initialize` and before any other requests

## Pre-completion Checklist
Before declaring work done, run in order:
1. `cargo fmt` — format code
2. `cargo clippy -- -D warnings` — lint with zero warnings
3. `cargo test` — all unit tests pass
4. `cargo build` — clean compilation
