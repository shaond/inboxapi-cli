---
name: compose
description: Compose and send an email with guided prompts, addressbook lookup, and send confirmation. Use when the user wants to write and send an email.
user-invocable: true
disable-model-invocation: true
argument-hint: [recipient]
---

# Compose Email

Guide the user through composing and sending an email safely.

## Steps

1. **Identify sender**: Call `mcp__inboxapi__whoami` to get the current account email address

2. **Resolve recipient**:
   - If `$ARGUMENTS` is provided, use it as the recipient hint
   - Call `mcp__inboxapi__get_addressbook` to check for matching contacts
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

5. **Safety checks**:
   - Warn if the recipient address matches the sender's own @inboxapi.ai address
   - Warn if the body is empty
   - Warn if the subject is empty

6. **Confirm**: Ask the user to confirm: "Send this email? (yes/no)"

7. **Send**: Call `mcp__inboxapi__send_email` with `to`, `subject`, and `body`

8. **Confirm delivery**: Report the result to the user

## Rules

- ALWAYS show a preview before sending
- ALWAYS ask for explicit confirmation before calling send_email
- NEVER send an email without the user confirming
- If the user cancels, acknowledge and do not send
