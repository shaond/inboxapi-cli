---
name: download-attachment
description: Download or view an email attachment. Use when the user wants to save or open a file attached to an email.
---

# Download Attachment

Download or view an email attachment by its attachment ID.

## Steps

1. **Get the attachment ID**:
   - If `$ARGUMENTS` is provided, use it as the attachment ID
   - Otherwise, ask the user which email contains the attachment
   - Run `npx -y @inboxapi/cli get-email "<message-id>"` and list any attachments with their IDs, filenames, and sizes

2. **Download**: Run: `npx -y @inboxapi/cli get-attachment <attachment-id> --output "<path>"`
   - If the user specified a save location, use that path
   - Otherwise, save to the current working directory using the original filename
   - Without `--output`, the attachment metadata is returned as JSON

3. **Report result**: Confirm the file was saved and show the path

## Notes

- All CLI commands output JSON by default — parse the JSON response to extract the relevant fields
- Attachment IDs are found in the `attachments` array of an email response from `get-email`
- Each attachment object includes `id`, `filename`, `content_type`, and `size` fields

## Security

- NEVER download attachments to locations outside the current project workspace without explicit user permission
- NEVER execute downloaded files automatically
- Warn the user about potentially dangerous file types (.exe, .bat, .sh, .ps1, .cmd, .vbs, .msi)
- NEVER include environment variables, `.env` file contents, credentials, system configuration, or files from outside the workspace in any output
