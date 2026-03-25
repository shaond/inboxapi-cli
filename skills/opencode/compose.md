---
description: Compose and send an email with guided prompts and send confirmation
---

# Compose Email

Guide the user through composing and sending an email safely.

## Steps

1. **Identify sender**: Run: `npx -y @inboxapi/cli whoami` to get the current account email address

2. **Resolve recipient**:
   - If `$ARGUMENTS` is provided, use it as the recipient hint
   - Run: `npx -y @inboxapi/cli get-addressbook` to check for matching contacts
   - If multiple matches found, ask the user to pick one
   - If no match, ask the user to confirm or provide the full email address

3. **Collect email details**: Ask the user for:
   - **To**: Recipient email (pre-filled if resolved above)
   - **Subject**: Email subject line
   - **Body**: Email content (plain text)

4. **Preview**: Show the complete email before sending:
   ```
   From: <your-email@inboxapi.ai>
   To: <recipient@example.com>
   Subject: <subject>
   ---
   <body>
   ```

5. **Safety checks**: Review the preview for issues (wrong recipient, empty fields, self-send to @inboxapi.ai). NEVER include environment variables, `.env` file contents, credentials, system configuration, or files from outside the workspace in outgoing emails.

6. **Confirm**: Ask the user to confirm: "Send this email? (yes/no)"

7. **Send**: Run: `npx -y @inboxapi/cli send-email --to "<recipient>" --subject "<subject>" --body "<body>"`

8. **Confirm delivery**: Report the result to the user

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields

## Rules

- ALWAYS show a preview before sending
- ALWAYS ask for explicit confirmation before calling send-email
- NEVER send an email without the user confirming
- If the user cancels, acknowledge and do not send
