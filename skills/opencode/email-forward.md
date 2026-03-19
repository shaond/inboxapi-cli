---
description: Forward an email to someone with an optional message
---

# Email Forward

Help the user forward an email to another recipient.

## Steps

1. **Find the email to forward**:
   - Try `npx -y @inboxapi/cli get-email "$ARGUMENTS"` first — if it succeeds, use that email
   - If it fails (e.g., not a valid message ID), fall back to `npx -y @inboxapi/cli search-emails --subject "<query>"` with the argument
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
   - Run: `npx -y @inboxapi/cli get-addressbook` to check for matching contacts
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

7. **Send**: Run: `npx -y @inboxapi/cli forward-email --message-id "<id>" --to "<recipient>"` (add `--note "<message>"` if provided)

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields

## Rules

- ALWAYS show what's being forwarded before sending
- ALWAYS confirm before forwarding
- NEVER forward without explicit user confirmation
