# InboxAPI CLI

## Overview
Rust-based STDIO proxy that bridges JSON-RPC (MCP protocol) over STDIO to the remote InboxAPI MCP service over Streamable HTTP/SSE. Distributed via npm as `@inboxapi/cli`.

## Commands
- `cargo build` — build the Rust binary
- `cargo test` — run tests
- `cargo run -- proxy` — start the STDIO proxy (default)
- `cargo run -- login` — authenticate and store credentials
- `cargo run -- whoami` — show current account info
- `cargo run -- reset` — delete stored credentials (with optional backup prompt)
- `cargo run -- backup <folder>` — back up credentials to a folder
- `cargo run -- restore <folder>` — restore credentials from a backup folder
- `cargo run -- send-email --to user@example.com --subject "Hi" --body "Hello"` — send an email
- `cargo run -- send-email --to user@example.com --subject "Newsletter" --body-file ./body.txt --html-body-file ./newsletter.html` — send a file-backed text + HTML email
- `cargo run -- get-emails --limit 5` — list inbox emails
- `cargo run -- get-email "<message-id>"` — get a single email
- `cargo run -- search-emails --subject "keyword"` — search emails
- `cargo run -- get-attachment <id> --output ./file.pdf` — download an attachment
- `cargo run -- send-reply --message-id "<id>" --body "Reply"` — reply to an email (optional: `--body-file`, `--cc`, `--bcc`, `--reply-all`, `--html-body`, `--html-body-file`, `--from-name`, `--priority`)
- `cargo run -- forward-email --message-id "<id>" --to user@example.com` — forward an email
- `cargo run -- get-last-email` — get the most recent email
- `cargo run -- get-email-count` — get inbox email count (optional `--since`)
- `cargo run -- get-sent-emails --limit 10` — list sent emails
- `cargo run -- get-thread --message-id "<id>"` — get an email thread
- `cargo run -- get-addressbook` — get address book contacts
- `cargo run -- get-announcements` — get InboxAPI announcements
- `cargo run -- auth-introspect` — introspect current access token
- `cargo run -- auth-revoke --token "<token>"` — revoke a specific token
- `cargo run -- auth-revoke-all` — revoke all tokens
- `cargo run -- account-recover --name "x" --email "y"` — recover a lost account
- `cargo run -- verify-owner --email "x"` — verify email ownership
- `cargo run -- enable-encryption` — enable email encryption
- `cargo run -- reset-encryption` — reset email encryption
- `cargo run -- rotate-encryption --old-secret "x" --new-secret "y"` — rotate encryption secret

For complex HTML, templates, or large generated content, prefer `--body-file` and `--html-body-file`. File-backed bodies are validated as UTF-8 text, normalized to `\n`, and capped at 20 MiB.
- `cargo run -- setup-skills` — install skills for detected AI agents (interactive)
- `cargo run -- setup-skills --all` — install skills for all agents (Claude, Codex, Gemini, OpenCode)
- `cargo run -- setup-skills --claude --codex` — install for specific agents
- `cargo run -- setup-skills --force` — overwrite existing files
- `cargo run -- help` — show CLI help with examples

## Architecture
- **Single-file proxy** (`src/main.rs`): reads JSON-RPC from stdin, POSTs to remote endpoint, streams SSE responses to stdout. Injects stored access tokens into `tools/call` arguments.
- **npm wrapper** (`index.js`): resolves the platform-specific binary from `@inboxapi/cli-<os>-<arch>` optional dependencies, falls back to local `target/` builds or `cargo run`.
- **CI** (`.github/workflows/release.yml`): builds 5 platform binaries on tag push, creates GitHub Release, publishes platform + root npm packages.

## npm Distribution
```
@inboxapi/cli              — root package (index.js wrapper + optionalDependencies)
@inboxapi/cli-darwin-arm64 — macOS ARM64 binary
@inboxapi/cli-darwin-x64   — macOS x64 binary
@inboxapi/cli-linux-x64    — Linux x64 binary
@inboxapi/cli-linux-arm64  — Linux ARM64 binary
@inboxapi/cli-win32-x64    — Windows x64 binary
```

## Key Files
- `src/main.rs` — proxy logic, token injection, login flow, hashcash PoW
- `skills/claude/*/SKILL.md` — Claude Code skill definitions (7 skills)
- `skills/codex/*/SKILL.md` — Codex CLI skill definitions (7 skills)
- `skills/gemini/*/SKILL.md` — Gemini CLI skill definitions (7 skills)
- `skills/opencode/*.md` — OpenCode command definitions (7 commands)
- `skills/hooks/*.js` — Claude Code hook scripts (3 hooks)
- `index.js` — npm binary resolver (platform package → local build → cargo run)
- `package.json` — root npm package with optionalDependencies
- `npm/cli-*/package.json` — platform-specific npm package manifests
- `.github/workflows/release.yml` — cross-build + GitHub Release + npm publish
- `Cargo.toml` — Rust dependencies

## Pre-completion Checklist
Before declaring work done, run these in order:
1. `cargo fmt` — format code
2. `cargo clippy -- -D warnings` — lint with zero warnings
3. `cargo test` — all unit tests pass
4. `cargo build` — clean compilation

## Rules
- Do not add AI attribution to commits, code, or comments
- Run `cargo fmt` before committing Rust changes
- Use conventional commits (`feat:`, `fix:`, `chore:`, etc.)
