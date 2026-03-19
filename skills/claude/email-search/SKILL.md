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

2. Translate the natural language query into CLI flags for `search-emails`:
   - Extract sender hints (e.g., "from John" -> `--sender "John"`)
   - Extract subject hints (e.g., "about invoices" -> `--subject "invoices"`)
   - Extract date hints (e.g., "last week", "yesterday" -> `--since "..."`, `--until "..."`)
   - Combine with `--limit` as needed

3. Run: `npx -y @inboxapi/cli search-emails` with the appropriate flags (`--sender "..."`, `--subject "..."`, `--since "..."`, `--until "..."`)

4. Present results in a formatted table:
   ```
   | # | From | Subject | Date |
   |---|------|---------|------|
   ```

5. After showing results, offer: "Would you like to read any of these emails? Provide the number."

6. If the user picks one, run `npx -y @inboxapi/cli get-email "<message-id>"` with the email ID

7. If no results, suggest alternative searches or broader terms

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields

## Examples

- `/email-search invoices from accounting` -> search for "invoices" filtered by sender containing "accounting"
- `/email-search meeting tomorrow` -> search for "meeting" in recent emails
- `/email-search` -> prompt user for search query
