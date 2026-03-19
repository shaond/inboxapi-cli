---
name: check-inbox
description: "Check your InboxAPI email inbox and display a summary of recent messages. Use when the user wants to see their emails, check mail, or view their inbox."
---

# Check Inbox

Fetch and display a summary of recent emails from the user's InboxAPI inbox.

## Steps

1. Run: `npx -y @inboxapi/cli whoami` to identify the current account and email address
2. Run: `npx -y @inboxapi/cli get-email-count` to show the total number of emails
3. Run: `npx -y @inboxapi/cli get-emails --limit <N>` where `<N>` is `$ARGUMENTS` if provided, otherwise `20`
4. Present results in a formatted table with columns:
   - **From** — sender name or address
   - **Subject** — email subject line (truncated to 60 chars)
   - **Date** — received date in relative format (e.g., "2 hours ago", "yesterday")
5. After the table, show a summary line: "Showing X of Y emails for <email>"

## Output Format

Use this markdown table format:

```
| # | From | Subject | Date |
|---|------|---------|------|
| 1 | Alice <alice@example.com> | Re: Project update... | 2h ago |
```

If the inbox is empty, display: "Your inbox is empty. Your email address is <email>."

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields
- Do NOT read full email bodies — only show the summary list
- If the user asks to read a specific email after seeing the list, run `npx -y @inboxapi/cli get-email "<message-id>"` with the email ID
