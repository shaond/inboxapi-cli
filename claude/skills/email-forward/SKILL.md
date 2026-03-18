---
name: email-forward
description: Forward an email to someone with an optional message. Use when the user wants to forward a specific email to another person.
user-invocable: true
disable-model-invocation: true
argument-hint: [email-id or subject]
---

# Email Forward

Help the user forward an email to another recipient.

## Steps

1. **Find the email to forward**:
   - If `$ARGUMENTS` looks like an email ID, call `mcp__inboxapi__get_email` directly
   - Otherwise, call `mcp__inboxapi__search_emails` with the argument
   - If multiple results, show them and ask the user to pick one

2. **Show email content**: Display the email being forwarded:
   ```
   --- Email to forward ---
   From: <original sender>
   Subject: <subject>
   Date: <date>
   ---
   <body preview, first 500 chars>
   ```

3. **Resolve recipient**:
   - Ask "Who do you want to forward this to?"
   - Call `mcp__inboxapi__get_addressbook` to check for matching contacts
   - Confirm the recipient email address

4. **Optional message**: Ask "Add a message? (or press enter to skip)"

5. **Preview**:
   ```
   Forwarding to: <recipient>
   Subject: Fwd: <original subject>
   Your message: <optional message or "(none)">
   Original email from: <sender>, <date>
   ```

6. **Confirm**: Ask "Forward this email? (yes/no)"

7. **Send**: Call `mcp__inboxapi__forward_email` with the email ID, recipient, and optional message

## Rules

- ALWAYS show what's being forwarded before sending
- ALWAYS confirm before forwarding
- NEVER forward without explicit user confirmation
