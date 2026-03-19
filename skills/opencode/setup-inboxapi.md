---
description: Set up InboxAPI email tools for your AI coding agent
---

# Setup InboxAPI

Configure InboxAPI email tools for this project.

## Steps

1. **Check current setup**: Look for existing MCP server configuration files

2. **Add MCP server** (if not already configured):
   - For OpenCode: Add to `opencode.json` under `mcp`:
     ```json
     {
       "mcp": {
         "inboxapi": {
           "command": "npx",
           "args": ["-y", "@inboxapi/cli"]
         }
       }
     }
     ```

3. **Install skills**: Run `npx -y @inboxapi/cli setup-skills` to copy bundled skills into the project

4. **Verify credentials**:
   - Run: `npx -y @inboxapi/cli whoami` to check if credentials are set up
   - If not authenticated, instruct the user: "Run `npx -y @inboxapi/cli login` in a terminal to authenticate"

5. **Show summary**:
   ```
   InboxAPI Setup Complete!

   MCP Server: configured
   Email: <email> (or "not authenticated yet")

   Installed Skills:
     /check-inbox  — View inbox summary
     /compose      — Write and send emails
     /email-search — Search emails
     /email-reply  — Reply with thread context
     /email-digest — Email activity digest
     /email-forward — Forward emails

   Next steps:
     - Run /check-inbox to see your emails
     - Run /compose to send your first email
   ```

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields
- This skill is safe to run multiple times — it won't duplicate entries or overwrite local edits
