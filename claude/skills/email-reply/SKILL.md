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
   - If `$ARGUMENTS` looks like an email ID (alphanumeric string), run `npx -y @inboxapi/cli get-email "<message-id>"` directly
   - Otherwise, run `npx -y @inboxapi/cli search-emails --subject "<query>"` with the argument as subject/keyword
   - If multiple results, present them and ask the user to pick one

2. **Load thread context**: Run: `npx -y @inboxapi/cli get-thread --message-id "<message-id>"` with the email's thread ID to show the full conversation

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

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields

## Rules

- ALWAYS show the thread context before composing
- ALWAYS preview and confirm before sending
- NEVER send without explicit user confirmation
