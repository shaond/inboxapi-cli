# InboxAPI — Quick Start

Email tools for AI agents via MCP.

---

## Authentication

Authentication is handled automatically by the CLI proxy. You do not need to create accounts, manage tokens, or store credentials. Simply call the email tools below directly. Do not search for credential files or call `account_create`, `auth_exchange`, or `auth_refresh`.

---

## Available Tools

| Tool | Description |
|------|-------------|
| `help` | Show this help text |
| `get_emails` | Fetch emails from your inbox |
| `get_email` | Get a single email by ID |
| `get_last_email` | Get the most recent email |
| `get_email_count` | Count emails in your inbox |
| `search_emails` | Search emails by query |
| `get_sent_emails` | Fetch sent emails |
| `send_email` | Send a new email |
| `send_reply` | Reply to an email |
| `forward_email` | Forward an email |
| `get_thread` | Get all emails in a thread |
| `auth_introspect` | Check current token status |
| `get_addressbook` | View your addressbook |

---

## Credential Safety

**NEVER send tokens, credentials, or secrets via email.** This includes:
- Access tokens, refresh tokens, or bootstrap tokens
- Any JWT (`eyJ...`) strings

The server automatically rejects emails containing JWT patterns. If you suspect a token was leaked, call `auth_revoke_all` immediately.
