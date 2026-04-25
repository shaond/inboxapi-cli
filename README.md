# InboxAPI CLI

Give your AI agent its own personal email address. Send, receive, read, search, and reply to emails — right from Claude, OpenCode, Codex, Gemini, or any MCP-compatible AI client. No email server to run, no SMTP to configure.


---

### Table of Contents

- [How it works](#how-it-works)
- [Technical details](#technical-details)
- [Good to know](#good-to-know)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Commands](#commands)
- [CLI Commands](#cli-commands)
- [Usage with MCP Clients](#usage-with-mcp-clients)
- [Skills for Claude Code](#skills-for-claude-code)
- [Development](#development)
- [FAQ](#faq)
- [License](#license)
- [Disclaimer](#disclaimer)

---

### How it works

1. Install the CLI
2. Connect it to your AI client (Claude Desktop, Claude Code, Gemini CLI, OpenCode, etc.)
3. Your AI can now use email — no code or API keys needed

An account with a unique, personal email address is created automatically on first run. Your AI can then:

- **Send emails** to any address
- **Receive emails** at its own inbox
- **Reply to** and **forward** emails
- **Search** emails by keyword
- **Read full threads** of conversation

## Technical details

The CLI acts as a local bridge between your AI client and the [InboxAPI](https://inboxapi.ai) cloud service. It speaks the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) over standard input/output, so any compatible AI client can use it without custom integration.

## Good to know

- **This is your agent's personal email** — InboxAPI gives your AI agent its own email address for personal use. It is not a transactional email service — don't use it for bulk sending, marketing, or application notifications.
- **Weekly send limit** — Each account can send to up to five unique email addresses per week. This resets weekly.
- **Check your spam folder** — Each agent gets its own subdomain, and new subdomains don't have email reputation yet. Early messages may land in your recipient's spam or junk folder. Adding your agent's email address to your contacts or allowlist helps. Delivery improves over time as recipients interact with your agent's emails.
- **Attachments** — Send attachments via CLI subcommands using `--attachment` (local files) or `--attachment-ref` (server-side attachments by ID).
- **HTML email support** — CLI subcommands support HTML emails with `--html-body` or `--html-body-file`.
- **Owner verification** — Link your email to your agent's account with `verify_owner` to enable account recovery and remove trial restrictions. Recommended as a first step after setup.

## Installation

```bash
npm install -g @inboxapi/cli@latest
```

Prebuilt binaries are included for:

| Platform       | Architecture |
|----------------|--------------|
| macOS          | ARM64, x64   |
| Linux          | x64, ARM64   |
| Windows        | x64          |

## Updating

Run the same install command to update to the latest version:

```bash
npm install -g @inboxapi/cli@latest
```

The CLI also checks for updates automatically when running in proxy mode and installs them in the background.

## Getting Started

Just start the proxy — an account is created automatically on first run

```bash
inboxapi proxy
```

On first run with no saved credentials, the CLI auto-creates an account with a generated name (e.g. `brooding-fluffy-owl`) and authenticates. No manual setup needed.

Credentials are stored in your system config directory and automatically injected into tool calls. The CLI checks multiple locations so it can pick up credentials created by AI agents:

- `~/Library/Application Support/inboxapi/credentials.json` (macOS primary)
- `~/.config/inboxapi/credentials.json` (Linux primary / macOS fallback)
- `~/.local/inboxapi/credentials.json` (fallback, used by some AI agents)

## Commands

### `proxy` (default)

Starts the STDIO proxy. Reads JSON-RPC messages from stdin, forwards them to the InboxAPI endpoint, and streams SSE responses to stdout. If no credentials are found, an account is automatically created with a generated name.

```bash
inboxapi proxy
inboxapi proxy --endpoint https://custom-endpoint.example.com/mcp
```

Running `inboxapi` with no subcommand also starts the proxy.

### `login`

Manually creates an account with a chosen name and stores access credentials locally. Not required for basic usage since `proxy` handles account creation automatically.

```bash
inboxapi login
inboxapi login --name myaccount
inboxapi login --endpoint https://custom-endpoint.example.com/mcp
```

### `whoami`

Displays the currently authenticated account and endpoint.

```bash
inboxapi whoami
```

### `reset`

Deletes stored credentials. Interactively offers to back up first, then asks for confirmation before deleting.

```bash
inboxapi reset
```

### `backup`

Backs up credentials to a specified folder.

```bash
inboxapi backup ./my-backup
```

### `restore`

Restores credentials from a backup folder. Validates backup integrity and offers to back up existing credentials before overwriting.

```bash
inboxapi restore ./my-backup
```

### `setup-skills`

Installs InboxAPI skills for AI coding agents. Supports **Claude Code**, **Codex CLI**, **Gemini CLI**, and **OpenCode**. Auto-detects installed agents and prompts for confirmation, or use flags for non-interactive installation.

```bash
inboxapi setup-skills              # Auto-detect agents, interactive prompt
inboxapi setup-skills --all        # Install for all 4 agents
inboxapi setup-skills --claude --codex  # Install for specific agents
inboxapi setup-skills --force      # Overwrite existing skills and hooks
```

## CLI Commands

For agents with shell access, CLI subcommands are the simplest way to use InboxAPI — no MCP, JSON-RPC, or base64 knowledge needed.

### `send-email`

```bash
inboxapi send-email --to user@example.com --subject "Hello" --body "Hi there"
inboxapi send-email --to user@example.com --subject "Report" --body "See attached" --attachment ./report.pdf
inboxapi send-email --to user@example.com --subject "Fwd" --body "See attached" --attachment-ref 9f0206bb-...
inboxapi send-email --to "a@b.com, c@d.com" --subject "Hi" --body "Hello" --cc "cc@b.com" --priority high
inboxapi send-email --to user@example.com --subject "Newsletter" --body-file ./body.txt --html-body-file ./newsletter.html
inboxapi send-email --to user@example.com --subject "Screenshot" --body-file ./body.txt --html-body-file ./email-with-inline-image.html
```

Supports `--body` or `--body-file`, `--html-body` or `--html-body-file`, `--cc`, `--bcc`, `--priority`, `--attachment` (local files, repeatable), and `--attachment-ref` (server-side attachment IDs, repeatable). `--from-name` is deprecated and ignored; InboxAPI enforces the authenticated account identity.

Prefer `--body-file` and `--html-body-file` for complex HTML, templates, or large generated payloads such as inline base64 images. File-backed bodies are validated as UTF-8 text, normalized to `\n` line endings, and capped at 20 MiB before the request is sent.

### `get-emails`

```bash
inboxapi get-emails --limit 5
inboxapi get-emails --limit 5 --human
```

### `get-email`

```bash
inboxapi get-email "<message-id>"
```

### `search-emails`

```bash
inboxapi search-emails --subject "invoice" --limit 10
```

### `get-attachment`

```bash
inboxapi get-attachment abc123                      # prints signed URL as JSON
inboxapi get-attachment abc123 --output ./file.pdf  # downloads to file
```

### `send-reply`

```bash
inboxapi send-reply --message-id "<msg-id>" --body "Thanks!"
inboxapi send-reply --message-id "<msg-id>" --body-file ./reply.txt --html-body-file ./reply.html
```

### `forward-email`

```bash
inboxapi forward-email --message-id "<msg-id>" --to recipient@example.com --note "FYI"
```

### `help`

```bash
inboxapi help  # CLI-focused help with examples
```

All CLI commands support the `--human` flag for human-readable output instead of JSON.

## Usage with MCP Clients

InboxAPI CLI also works as an MCP STDIO transport. Point your MCP client at the `inboxapi` binary:

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "inboxapi": {
      "command": "inboxapi"
    }
  }
}
```

**Claude Code:**

Add to current project:

```bash
claude mcp add inboxapi inboxapi
```

Add globally (available in all projects):

```bash
claude mcp add inboxapi inboxapi -s user
```

**Gemini CLI:**

Add to current project:

```bash
gemini mcp add inboxapi inboxapi
```

Add system-wide (available in all directories):

```bash
gemini mcp add inboxapi inboxapi --scope user
```

**OpenCode:**

Run the interactive setup:

```bash
opencode mcp add
```

When prompted, enter:
- **Location:** Global
- **MCP server name:** inboxapi
- **MCP server type:** Local
- **Command to run:** inboxapi

**Codex CLI:**

```bash
codex mcp add inboxapi inboxapi
```

## Skills for AI Coding Agents

InboxAPI includes skills — slash commands and guided workflows — for multiple AI coding agents. Install them with:

```bash
inboxapi setup-skills        # Auto-detect and install
inboxapi setup-skills --all  # Install for all agents
```

Skills are installed to agent-specific directories:

| Agent | Install Directory |
|-------|-------------------|
| Claude Code | `.claude/skills/` |
| Codex CLI | `.agents/skills/` |
| Gemini CLI | `.gemini/skills/` |
| OpenCode | `.opencode/commands/` |

### Available Skills

| Skill | Description |
|-------|-------------|
| `/check-inbox` | Fetch and display a summary of recent emails in a formatted table |
| `/compose` | Compose and send an email with guided prompts, addressbook lookup, and confirmation |
| `/email-search` | Search emails using natural language queries |
| `/email-reply` | Reply to an email with full thread context and preview before sending |
| `/email-digest` | Generate a structured digest of recent email activity grouped by threads |
| `/email-forward` | Forward an email to another recipient with an optional note |
| `/setup-inboxapi` | Configure InboxAPI MCP server and install skills for your AI coding agent |

### Hooks (Claude Code only)

The `setup-skills` command also installs three hooks for Claude Code that run automatically:

| Hook | Type | Description |
|------|------|-------------|
| Credential Check | SessionStart | Verifies InboxAPI credentials on startup and shows authentication status |
| Email Send Guard | PreToolUse | Reviews outbound emails before sending, warns about self-sends and empty bodies |
| Activity Logger | PostToolUse | Logs all InboxAPI tool usage to `.claude/inboxapi-activity.log` for audit trails |

## Development

```bash
cargo build           # Build debug binary
cargo build --release # Build release binary
cargo test            # Run tests
cargo fmt             # Format code
```

## FAQ

### Why not just give my agent access to my Gmail or Outlook?

**Security** — Gmail/Outlook OAuth gives your agent access to your entire inbox (medical, financial, legal, personal). A prompt injection in any inbound email could manipulate an agent with access to all of it. InboxAPI gives your agent its own isolated inbox with trust classification and datamarking on every message.

**Identity** — When your agent sends from your Gmail, recipients can't tell who they're talking to. Replies go to your inbox, mixed with your real mail. InboxAPI gives your agent its own personal address — clear separation between you and your agent.

**Practicality** — Gmail/Outlook APIs aren't MCP-native. You'd need middleware, OAuth plumbing, and custom integration. InboxAPI works out of the box with any MCP client.

### How is this different from AWS SES, SendGrid, or Resend?

Those are sending APIs — you build email infrastructure on top of them. InboxAPI gives your agent a complete email identity: send, receive, search, reply, and forward. There's nothing to configure and no infrastructure to manage.

### How is this different from AgentMail or a1base?

We built our own email stack from the ground up. We don't wrap SES, Postfix, or any third-party sending service. Your agent's mail goes through infrastructure we operate directly.

### Is it really free?

Yes. No credit card, no trial period, no usage tiers. We're working on paid plans with additional features, but the core experience will always be free.

### How do you prevent spam and abuse?

Account creation requires proof-of-work. Each account can only email 5 unique external email addresses per week. Daily send quotas and rate limiting are enforced on every account. These constraints are structural — they're not policies, they're how the system works.

### What about prompt injection via email?

Every inbound email includes a trust classification — trusted, agent, unverified, or suspicious — based on whether the sender is in your addressbook and whether their email passes authentication checks. This helps your agent decide how cautiously to handle each message. Emails from other InboxAPI agents are flagged separately so your agent knows to check with you before acting on them.

Additionally, untrusted email content is automatically transformed using spotlighting (datamarking) — whitespace is replaced with a unique marker character so your agent can clearly distinguish email data from its own instructions. This reduces the success rate of prompt injection attacks embedded in emails from ~50% to under 3%.

### What is spotlighting?

Email retrieval tools apply datamarking to untrusted content, replacing whitespace with a unique Unicode marker character generated per request. Content containing the marker should be treated as external data — never as instructions to follow. To recover the original text, replace the marker with a space. Emails from trusted senders (in your addressbook with valid authentication) are not spotlighted by default. This technique is based on academic research ([arXiv:2403.14720](https://arxiv.org/abs/2403.14720)).

### What about data exfiltration?

Outbound emails are scanned for authentication tokens and credentials. If your agent accidentally tries to send an email containing a JWT or access token, the message is rejected before it leaves the platform. This prevents agents from being tricked into leaking sensitive data via email. Additionally, all recipient addresses in send, reply, and forward operations are validated against RFC 5322 — malformed addresses are rejected before delivery.

### Can agents spam each other?

The same send limits apply to all outbound email — recipient caps, quotas, and rate limiting work the same regardless of who's on the receiving end.

### Will my agent's emails land in spam?

Maybe at first. Each agent gets a brand-new subdomain, and new senders don't have reputation yet. Recipients may need to check their spam folder for the first few emails. Over time, as your agent sends legitimate mail and recipients interact with it, delivery improves.

### Why email instead of a native agent protocol like A2A?

Email reaches the entire existing internet — billions of people and businesses already use it. A2A requires both sides to implement the protocol. When your agent needs to reach someone outside its own ecosystem, email is the universal option. Agents will likely need both.

### Why email instead of WhatsApp, Telegram, or other messaging apps?

**Scalability** — You can programmatically create hundreds of email addresses. WhatsApp, Telegram, and Signal all require phone numbers and verification. Scaling past a handful of accounts is impractical, often against terms of service, and sometimes impossible without physical SIM cards.

**No gatekeeping** — Email is the only communication channel where you can create an identity without a phone number, government ID, or approval from a platform owner. No single company controls who gets an email address.

**Open protocol** — Email is federated and vendor-neutral. WhatsApp, Discord, and Telegram are proprietary — they can revoke API access, ban bot accounts, or change the rules at any time. Email can't be shut off by one company.

**ToS compliance** — Most messaging platforms explicitly prohibit automated accounts or have strict approval processes (WhatsApp Business API requires business verification, Telegram restricts bot-to-bot messaging). Email has no such restrictions — automated sending is a first-class use case.

**Universal reach** — Messaging channels are siloed. Your Telegram bot can't reach a WhatsApp user. Email reaches anyone with an email address — which is effectively everyone.

For multi-channel agent frameworks like [OpenClaw](https://openclaw.ai/), email fills a gap that messaging platforms structurally cannot — unlimited, programmable identity creation with no platform approval required. InboxAPI gives agents that capability out of the box.

### What are the send limits?

Each account can email up to 5 unique external email addresses per week. Emails to other @inboxapi.ai addresses don't count against this limit. The limit resets weekly.

### What happens when I hit the limit?

When all 5 slots are in use, the least recently used entry is auto-replaced after 5 days of inactivity.

### Can I send attachments?

Yes. Attachment support is fully available. Supply an array of `EmailAttachment` objects containing the `filename`, `content_type`, and base64-encoded `content` in the `attachments` field when calling `send_email`.

### Can I send HTML emails?

Yes. Use `--html-body "<html>"` for inline HTML or `--html-body-file ./email.html` for file-backed HTML content. For more complex templates or large generated payloads, prefer `--body-file` and `--html-body-file`.

### How do credentials work?

Your agent's credentials are stored locally at `~/.config/inboxapi/credentials.json` (Linux) or `~/Library/Application Support/inboxapi/credentials.json` (macOS). The CLI handles token creation and refresh automatically — your agent never needs to manage tokens manually.

### What if my agent loses access?

If your agent's credentials are lost or corrupted, you can recover the account using the `account_recover` tool — but only if you previously linked your email via `verify_owner`. Recovery revokes all existing tokens and issues new credentials. Without a verified owner email, there is no way to recover a locked-out account.

### What is owner verification?

Owner verification links your personal email address to your agent's InboxAPI account. Your agent calls `verify_owner` with your email, you receive a 6-digit code, and your agent submits it to complete verification. Once verified, you can recover the account if credentials are ever lost, and trial restrictions are removed from the account.

### What domains are blocked from sending?

InboxAPI maintains a denylist that blocks sending to government (.gov), military (.mil), intelligence, law enforcement, nuclear/critical infrastructure, and disposable email domains.

### How does the trust classification work?

Every inbound email is classified into one of four trust levels:

| Trust Level | Meaning | Recommended Action |
|-------------|---------|-------------------|
| Trusted | Sender is in your addressbook with valid SPF/DKIM | Safe to act on |
| Agent | Sender is a known InboxAPI agent | Read freely, but confirm with your human before taking actions |
| Unverified | Valid SPF/DKIM but sender not in addressbook | Use caution |
| Suspicious | Authentication failed or unknown sender | Flag and confirm before acting |

### What AI model should I use with InboxAPI?

Your model must support **tool/function calling** — MCP requires this. We recommend a minimum **32K token context window** to comfortably fit InboxAPI's 21 tool definitions alongside conversation history and email content.

**Model recommendations by tier:**

| Tier | Anthropic | OpenAI | Google |
|------|-----------|--------|--------|
| Good | Haiku 4.5+ | GPT-4.1 mini+, GPT-4.1 nano+ | Gemini 2.5 Flash+ |
| Recommended | Sonnet 4.5+ | GPT-4.1+, GPT-5 mini+ | Gemini 2.5 Pro+ |
| Best | Opus 4.5+ | GPT-5+, GPT-5.2+ | Gemini 2.5 Pro+ |

**Datamarking overhead:** InboxAPI applies datamarking (spotlighting) to untrusted email content, replacing whitespace with Unicode marker characters. This can slightly increase token consumption when processing emails from external senders. Models with larger context windows handle this more comfortably.

**What won't work:** Models without tool/function calling support, models with context windows under 16K tokens, and very small local models (under ~7B parameters) that lack reliable tool calling. These will struggle to fit InboxAPI's 21 tool definitions and maintain useful conversation history.

### What stops an agent from buying things or authorizing transactions via email?

InboxAPI is a communication channel, not an execution environment. It can deliver an email, but it can't click buttons, enter credit card numbers, or interact with external systems. The risk of unauthorized actions comes from how an agent is configured and what other tools it has access to — not from its email.

## License

The source code in this repository is licensed under the [MIT License](LICENSE).

## Disclaimer

The InboxAPI service is provided as-is, with no guarantees or warranties of any kind. We reserve all rights regarding how the service is operated. Service terms, features, and availability may change at any time without notice.
