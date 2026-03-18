---
name: email-digest
description: Generate a digest summary of recent email activity grouped by thread. Use when the user wants an overview of their email activity.
user-invocable: true
argument-hint: [timeframe]
---

# Email Digest

Generate a structured digest of recent email activity.

## Steps

1. **Determine timeframe**: Use `$ARGUMENTS` if provided (e.g., "today", "this week", "last 3 days"), otherwise default to "last 24 hours"

2. **Get account info**: Call `mcp__inboxapi__whoami` for the account email

3. **Get total count**: Call `mcp__inboxapi__get_email_count` for inbox statistics

4. **Fetch recent emails**: Call `mcp__inboxapi__get_emails` with an appropriate limit (50 for digest)

5. **Group by thread**: For threads with multiple emails, call `mcp__inboxapi__get_thread` to understand the conversation

6. **Generate digest** with these sections:

   ```markdown
   # Email Digest — <timeframe>
   Account: <email>

   ## Summary
   - Total emails in inbox: X
   - Emails in this period: Y
   - Unique senders: Z
   - Threads with activity: N

   ## Active Threads
   ### 1. <Subject>
   - Participants: alice@..., bob@...
   - Messages in period: 3
   - Latest: "Brief preview of most recent message..."
   - Status: Awaiting your reply / You replied / FYI only

   ## New Emails (not in threads)
   | From | Subject | Date |
   |------|---------|------|
   | ... | ... | ... |

   ## Needs Attention
   - Emails you haven't replied to where you were directly addressed
   ```

7. **Offer actions**: "Would you like to reply to any of these, or read a specific email?"

## Notes

- Focus on actionable insights, not raw data
- Highlight emails that likely need a response
- Keep the digest concise — summarize, don't reproduce full emails
