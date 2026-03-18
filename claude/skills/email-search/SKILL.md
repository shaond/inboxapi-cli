---
name: email-search
description: Search your InboxAPI emails using natural language. Use when the user wants to find specific emails by sender, subject, date, or content.
user-invocable: true
argument-hint: [query]
---

# Email Search

Search emails using natural language and present results clearly.

## Steps

1. Take the user's query from `$ARGUMENTS`
   - If no arguments provided, ask: "What are you looking for?"

2. Translate the natural language query into a `mcp__inboxapi__search_emails` call:
   - Extract sender hints (e.g., "from John" -> search by sender)
   - Extract subject hints (e.g., "about invoices" -> search by subject)
   - Extract date hints (e.g., "last week", "yesterday")
   - Use the full query as the search term

3. Call `mcp__inboxapi__search_emails` with the interpreted parameters

4. Present results in a formatted table:
   ```
   | # | From | Subject | Date |
   |---|------|---------|------|
   ```

5. After showing results, offer: "Would you like to read any of these emails? Provide the number."

6. If the user picks one, call `mcp__inboxapi__get_email` with the email ID

7. If no results, suggest alternative searches or broader terms

## Examples

- `/email-search invoices from accounting` -> search for "invoices" filtered by sender containing "accounting"
- `/email-search meeting tomorrow` -> search for "meeting" in recent emails
- `/email-search` -> prompt user for search query
