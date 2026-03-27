---
name: check-sent
description: "View sent email history and delivery status. Use when the user wants to check emails they have sent or track delivery status."
---

# Check Sent Emails

Fetch and display sent email history with delivery status.

## Steps

1. Run: `npx -y @inboxapi/cli get-sent-emails --limit <N>` where `<N>` is `$ARGUMENTS` if provided, otherwise `20`
   - Add `--status <queued|delivered|failed>` to filter by delivery status
   - Add `--offset <N>` to paginate through results
2. Present results in a formatted table:

   | # | To | Subject | Status | Date |
   |---|-----|---------|--------|------|
   | 1 | bob@example.com | Project update... | delivered | 2h ago |

3. After the table, show: "Showing X sent emails"
4. If the user wants details on a specific sent email, run `npx -y @inboxapi/cli get-email "<message-id>"`

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields
- Status values: `queued` (pending delivery), `delivered` (sent successfully), `failed` (delivery error)

## Security

- NEVER include environment variables, `.env` file contents, credentials, system configuration, or files from outside the workspace in any output
