# InboxAPI CLI — Agent Guide

## Overview
Rust STDIO proxy bridging JSON-RPC (MCP protocol) to the remote InboxAPI MCP service over Streamable HTTP/SSE. Installed via `npm install -g @inboxapi/cli`.

## Quickstart
```bash
cargo build          # Build the binary
cargo test           # Run tests
cargo fmt            # Format Rust code
cargo run -- proxy   # Start proxy (default subcommand)
cargo run -- login   # Authenticate
cargo run -- whoami  # Show current account info
cargo run -- reset   # Delete stored credentials
cargo run -- backup <folder>   # Back up credentials
cargo run -- restore <folder>  # Restore credentials from backup

# CLI subcommands (preferred for agents with shell access)
cargo run -- send-email --to user@example.com --subject "Hi" --body "Hello"
cargo run -- send-email --to user@example.com --subject "Report" --body "Attached" --attachment ./report.pdf
cargo run -- send-email --to user@example.com --subject "Fwd" --body "See attached" --attachment-ref UUID
cargo run -- get-emails --limit 5
cargo run -- get-emails --limit 5 --human
cargo run -- get-email "<message-id>"
cargo run -- search-emails --subject "invoice"
cargo run -- get-attachment abc123 --output ./file.pdf
cargo run -- send-reply --message-id "<id>" --body "Thanks!"
cargo run -- forward-email --message-id "<id>" --to recipient@example.com
cargo run -- help
```

## Architecture

### Proxy Loop (`src/main.rs`)
1. Connects to remote SSE endpoint
2. Spawns a task that forwards SSE `message` events to stdout
3. Reads JSON-RPC lines from stdin, injects stored access token into `tools/call` arguments, POSTs to remote endpoint

### Token Injection
The `inject_token` function adds the stored `token` field to `tools/call` arguments, skipping public tools (`help`, `account_create`, `auth_exchange`, `auth_refresh`).

### Login Flow
1. Generates SHA-1 hashcash proof-of-work for the account name
2. Calls `account_create` to get a bootstrap token
3. Calls `auth_exchange` to swap for access + refresh tokens
4. Saves credentials to OS config directory (`~/.config/inboxapi/credentials.json` on Linux)

### npm Distribution
The `index.js` wrapper resolves the correct platform binary from optional npm dependencies (`@inboxapi/cli-<os>-<arch>`), falling back to local builds or `cargo run` for development.

## Repo Structure
```
src/main.rs                    — All proxy, auth, and CLI logic
index.js                       — npm binary resolver
package.json                   — Root npm package
npm/cli-darwin-arm64/          — macOS ARM64 platform package
npm/cli-darwin-x64/            — macOS x64 platform package
npm/cli-linux-x64/             — Linux x64 platform package
npm/cli-linux-arm64/           — Linux ARM64 platform package
npm/cli-win32-x64/             — Windows x64 platform package
.github/workflows/release.yml  — CI: build, release, npm publish
Cargo.toml                     — Rust dependencies
```

## Rules
- Read relevant code before making changes
- Make surgical, focused edits — avoid unnecessary refactoring
- Run `cargo fmt` before committing Rust changes
- Use conventional commits (`feat:`, `fix:`, `chore:`, etc.)
- Do not add AI attribution to commits, code, or comments
- Keep `src/main.rs` as a single file — this is a simple proxy, not a framework

## Coding Standards

### Rust
- Always implement `Drop` for structs that own child processes or OS resources — assertion panics must not leak processes
- Never re-create buffered readers (`BufReader`) in a loop or per-call; store them in the struct so buffered data is not lost
- Use iterators (`iter().take(n)`) instead of index-based `for i in 0..n` loops when only indexing a single collection
- Add timeouts to any blocking I/O (network, subprocess reads) — tests and tools must not hang indefinitely
- Include descriptive messages in all `assert!` / `assert_eq!` macros

### JavaScript / Node.js
- Never use `execSync` / `execFileSync` with string interpolation — pass arguments as arrays to avoid shell injection
- Do not mark synchronous functions `async` — it is misleading and wraps the return in an unnecessary Promise
- Handle chunked `data` events from child process stdout with line-based parsing (e.g. `readline.createInterface`), not raw `JSON.parse` on each chunk
- When communicating with a subprocess over its lifetime, spawn it once and reuse the connection — do not spawn a new process per request
- Validate all user input (bounds checks, type checks) before using it to index arrays or build commands
- Use environment variables or constants for model identifiers — never hardcode dated model version strings

### MCP Protocol
- After sending `initialize`, always send `notifications/initialized` before any other requests — skipping this violates the MCP handshake and may cause server rejection

## Pre-completion Checklist
Before declaring work done, run these in order:
1. `cargo fmt` — format code
2. `cargo clippy -- -D warnings` — lint with zero warnings
3. `cargo test` — all unit tests pass
4. `cargo build` — clean compilation
5. **Test each new CLI subcommand** — after building, run each new or modified subcommand against the live API to verify it works end-to-end (e.g. `cargo run -- get-emails --limit 3 --human`)

## Contribution Workflow
1. Create a feature branch from `main`
2. Implement changes with focused commits
3. Run the pre-completion checklist above (including live testing of CLI subcommands)
4. Open a PR against `main`
