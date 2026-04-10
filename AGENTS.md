# CLI Agent Stub

Canonical workflow lives in `../AGENTS.md`.

Repo-local notes:

- Verification flow: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`,
  `cargo build`
- This repo owns the CLI binary, npm packaging, and agent-install setup logic
- Canonical service docs live in `../docs/services/cli/`
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
- Centralize model identifiers in a single constant or environment variable; avoid scattering hardcoded dated model version strings throughout the code

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
