---
name: setup-inboxapi
description: Set up InboxAPI email tools in your Claude Code project. Adds MCP server config, installs skills, and configures safety hooks. Use when the user wants to configure InboxAPI for their project.
user-invocable: true
disable-model-invocation: true
---

# Setup InboxAPI

Configure InboxAPI email tools for this Claude Code project.

## Steps

1. **Check current setup**: Look for existing `.mcp.json` and `.claude/settings.json` files

2. **Add MCP server** (if not already configured):
   - Check if `.mcp.json` exists and contains an `inboxapi` entry
   - If not, run: `claude mcp add inboxapi -- npx -y @inboxapi/cli`
   - Or create/update `.mcp.json` with:
     ```json
     {
       "mcpServers": {
         "inboxapi": {
           "command": "npx",
           "args": ["-y", "@inboxapi/cli"]
         }
       }
     }
     ```

3. **Install skills**: Run `npx -y @inboxapi/cli setup-skills` to copy bundled skills and hooks into the project's `.claude/` directory

4. **Verify credentials**:
   - Call `mcp__inboxapi__whoami` to check if credentials are set up
   - If not authenticated, instruct the user: "Run `npx -y @inboxapi/cli login` in a terminal to authenticate"

5. **Show summary**:
   ```
   InboxAPI Setup Complete!

   MCP Server: configured in .mcp.json
   Email: <email> (or "not authenticated yet")

   Installed Skills:
     /check-inbox  — View inbox summary
     /compose      — Write and send emails
     /email-search — Search emails
     /email-reply  — Reply with thread context
     /email-digest — Email activity digest
     /email-forward — Forward emails

   Installed Hooks:
     PreToolUse  — Email send guard (reviews before sending)
     PostToolUse — Activity logger (audit trail)
     SessionStart — Credential check (verifies auth on startup)

   Next steps:
     - Run /check-inbox to see your emails
     - Run /compose to send your first email
   ```

## Notes

- This skill is safe to run multiple times — it won't duplicate entries or overwrite local edits
- Existing `.mcp.json` entries, skill files, and hook files with local edits are preserved
- `.claude/settings.json` is merged with new hook config (may be reformatted when hooks are updated)
- Files with local edits are skipped; unmodified files are reported as up to date
