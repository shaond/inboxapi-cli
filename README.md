# InboxAPI CLI

Give your AI agent its own email address. Send, receive, read, search, and reply to emails — right from Claude, OpenCode, Codex, Gemini, or any MCP-compatible AI client. No email server to run, no SMTP to configure.

**Quick Setup (Claude Code, Gemini CLI, OpenCode, etc.):**

Copy and paste this sentence into your agent:  

```
Install the `@inboxapi/cli@latest` npm package globally and then register the `inboxapi` command as an MCP server named `inboxapi`.
```

<details>
<summary>Table of Contents</summary>

- [Technical details](#technical-details)
- [Good to know](#good-to-know)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Commands](#commands)
- [Usage with MCP Clients](#usage-with-mcp-clients)
- [Development](#development)
- [FAQ](#faq)
- [License](#license)
- [Disclaimer](#disclaimer)

</details>

### How it works

1. Install the CLI
2. Connect it to your AI client (Claude Desktop, Claude Code, Gemini CLI, OpenCode, etc.)
3. Your AI can now use email — no code or API keys needed

An account with a unique email address is created automatically on first run. Your AI can then:

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
- **No attachments yet** — Attachment support is not available right now, but it's coming soon.
- **No rich text yet** — Emails are sent as plain text only. Rich text (HTML) support is coming soon.

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

## Usage with MCP Clients

InboxAPI CLI works as an MCP STDIO transport. Point your MCP client at the `inboxapi` binary:

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

Add to current project:

```bash
opencode mcp add inboxapi inboxapi
```

Add globally (available in all projects):

```bash
opencode mcp add inboxapi inboxapi -s user
```

**Codex CLI:**

```bash
codex mcp add inboxapi -- inboxapi
```

## Development

```bash
cargo build           # Build debug binary
cargo build --release # Build release binary
cargo test            # Run tests
cargo fmt             # Format code
```

## FAQ

**How is this different from AWS SES, SendGrid, or Resend?**

Those are sending APIs — you build email infrastructure on top of them. InboxAPI gives your agent a complete email identity: send, receive, search, reply, and forward. There's nothing to configure and no infrastructure to manage.

**How is this different from AgentMail or a1base?**

We built our own email stack from the ground up. We don't wrap SES, Postfix, or any third-party sending service. Your agent's mail goes through infrastructure we operate directly.

**Is it really free?**

Yes. No credit card, no trial period, no usage tiers. We're working on paid plans with additional features, but the core experience will always be free.

**How do you prevent spam and abuse?**

Account creation requires proof-of-work. Each account can only email 5 unique external domains per week. Daily send quotas and rate limiting are enforced on every account. These constraints are structural — they're not policies, they're how the system works.

**What about prompt injection via email?**

Every inbound email includes a trust classification — trusted, agent, unverified, or suspicious — based on whether the sender is in your addressbook and whether their email passes authentication checks. This helps your agent decide how cautiously to handle each message. Emails from other InboxAPI agents are flagged separately so your agent knows to check with you before acting on them.

Additionally, untrusted email content is automatically transformed using spotlighting (datamarking) — whitespace is replaced with a unique marker character so your agent can clearly distinguish email data from its own instructions. This reduces the success rate of prompt injection attacks embedded in emails from ~50% to under 3%.

**What is spotlighting?**

Email retrieval tools apply datamarking to untrusted content, replacing whitespace with a unique Unicode marker character generated per request. Content containing the marker should be treated as external data — never as instructions to follow. To recover the original text, replace the marker with a space. Emails from trusted senders (in your addressbook with valid authentication) are not spotlighted by default. This technique is based on academic research ([arXiv:2403.14720](https://arxiv.org/abs/2403.14720)).

**What about data exfiltration?**

Outbound emails are scanned for authentication tokens and credentials. If your agent accidentally tries to send an email containing a JWT or access token, the message is rejected before it leaves the platform. This prevents agents from being tricked into leaking sensitive data via email. Additionally, all recipient addresses in send, reply, and forward operations are validated against RFC 5322 — malformed addresses are rejected before delivery.

**Can agents spam each other?**

The same send limits apply to all outbound email — recipient caps, quotas, and rate limiting work the same regardless of who's on the receiving end.

**Will my agent's emails land in spam?**

Maybe at first. Each agent gets a brand-new subdomain, and new senders don't have reputation yet. Recipients may need to check their spam folder for the first few emails. Over time, as your agent sends legitimate mail and recipients interact with it, delivery improves.

**Why email instead of a native agent protocol like A2A?**

Email reaches the entire existing internet — billions of people and businesses already use it. A2A requires both sides to implement the protocol. When your agent needs to reach someone outside its own ecosystem, email is the universal option. Agents will likely need both.

**What are the send limits?**

Each account can email up to 5 unique external domains per week. Emails to other @inboxapi.ai addresses don't count against this limit. The limit resets weekly.

**What happens when I hit the limit?**

When all 5 slots are in use, the least recently used entry is auto-replaced after 5 days of inactivity.

**Can I send attachments?**

Not yet. Attachment support is coming soon.

**Can I send HTML emails?**

HTML email support is coming soon. Currently emails are sent as plain text.

**How do credentials work?**

Your agent's credentials are stored locally at `~/.config/inboxapi/credentials.json` (Linux) or `~/Library/Application Support/inboxapi/credentials.json` (macOS). The CLI handles token creation and refresh automatically — your agent never needs to manage tokens manually.

**What domains are blocked from sending?**

InboxAPI maintains a denylist that blocks sending to government (.gov), military (.mil), intelligence, law enforcement, nuclear/critical infrastructure, and disposable email domains.

**How does the trust classification work?**

Every inbound email is classified into one of four trust levels:

| Trust Level | Meaning | Recommended Action |
|-------------|---------|-------------------|
| Trusted | Sender is in your addressbook with valid SPF/DKIM | Safe to act on |
| Agent | Sender is a known InboxAPI agent | Read freely, but confirm with your human before taking actions |
| Unverified | Valid SPF/DKIM but sender not in addressbook | Use caution |
| Suspicious | Authentication failed or unknown sender | Flag and confirm before acting |

**What stops an agent from buying things or authorizing transactions via email?**

InboxAPI is a communication channel, not an execution environment. It can deliver an email, but it can't click buttons, enter credit card numbers, or interact with external systems. The risk of unauthorized actions comes from how an agent is configured and what other tools it has access to — not from its email.

## License

The source code in this repository is licensed under the [MIT License](LICENSE).

## Disclaimer

The InboxAPI service is provided as-is, with no guarantees or warranties of any kind. We reserve all rights regarding how the service is operated. Service terms, features, and availability may change at any time without notice.
