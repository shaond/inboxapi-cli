---
name: email-reply
description: Reply to an email with full thread context. Use when the user wants to reply to a specific email or continue an email conversation.
user-invocable: true
disable-model-invocation: true
argument-hint: [email-id or subject]
---

# Email Reply

Help the user reply to an email with full thread context.

## Steps

1. **Find the email**:
   - Try `npx -y @inboxapi/cli get-email "$ARGUMENTS"` first — if it succeeds, use that email
   - If it fails (e.g., not a valid message ID), fall back to `npx -y @inboxapi/cli search-emails --subject "<query>"` with the argument as subject/keyword
   - If multiple results, present them and ask the user to pick one

2. **Load thread context**: Run: `npx -y @inboxapi/cli get-thread --message-id "<message-id>"` with the email's message ID to show the full conversation

3. **Display thread**: Show the conversation history in chronological order:
   ```
   --- Thread: <subject> ---

   [1] From: alice@example.com (Jan 15, 2:30 PM)
   > Original message text...

   [2] From: you@inboxapi.ai (Jan 15, 3:00 PM)
   > Your previous reply...

   [3] From: alice@example.com (Jan 15, 4:15 PM)
   > Latest message you're replying to...
   ```

4. **Compose reply**: Ask the user what they want to say in their reply

5. **Preview**: Show the reply before sending:
   ```
   Replying to: alice@example.com
   Subject: Re: <subject>
   ---
   <reply body>
   ```

6. **Confirm**: Ask "Send this reply? (yes/no)"

7. **Send**: Run: `npx -y @inboxapi/cli send-reply --message-id "<id>" --body "<reply>"`

   **Preserving CC recipients in threads**: If the thread has CC'd recipients, include them with `--cc`:
   ```
   npx -y @inboxapi/cli send-reply --message-id "<id>" --body "<reply>" --cc "cc1@example.com,cc2@example.com"
   ```

   **Additional options**:
   - `--cc "addr1,addr2"` — CC recipients (comma-separated)
   - `--bcc "addr1,addr2"` — BCC recipients (comma-separated, silent copy)
   - `--reply-all` — reply to all recipients in the thread
   - `--html-body "<html>"` — send HTML-formatted reply
   - `--from-name "Name"` — override sender display name
   - `--priority <high|normal|low>` — set email priority

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields

## Rules

- ALWAYS show the thread context before composing
- ALWAYS preview and confirm before sending
- NEVER send without explicit user confirmation
- When replying to threads with CC'd recipients, ALWAYS preserve them using `--cc` to avoid breaking the chain
