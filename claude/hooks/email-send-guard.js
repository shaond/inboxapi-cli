#!/usr/bin/env node
// InboxAPI Email Send Guard — PreToolUse hook
// Reviews outbound emails before sending. Logs details to stderr for user visibility.
// Exit 0 = allow

const fs = require("fs");

function main() {
  const input = fs.readFileSync(0, "utf8");
  let data;
  try {
    data = JSON.parse(input);
  } catch {
    process.exit(0);
  }

  const toolName = data.tool_name || "";
  const toolInput = data.tool_input || {};

  let toDisplay, subject, body, action;

  if (toolName === "Bash") {
    // Check if this is an inboxapi CLI send command
    const cmd = (toolInput.command || "");
    if (!cmd.includes("inboxapi")) {
      process.exit(0);
    }
    const isSend = cmd.includes("send-email");
    const isReply = cmd.includes("send-reply");
    const isForward = cmd.includes("forward-email");
    if (!isSend && !isReply && !isForward) {
      process.exit(0);
    }

    // Best-effort extraction from CLI flags
    const toMatch = cmd.match(/--to\s+(?:"([^"]+)"|'([^']+)'|(\S+))/);
    toDisplay = (toMatch && (toMatch[1] || toMatch[2] || toMatch[3])) || "(unknown)";
    const subjectMatch = cmd.match(/--subject\s+(?:"([^"]+)"|'([^']+)'|(\S+))/);
    subject = (subjectMatch && (subjectMatch[1] || subjectMatch[2] || subjectMatch[3])) || "(no subject)";
    const bodyMatch = cmd.match(/--body\s+(?:"([^"]+)"|'([^']+)'|(\S+))/);
    body = (bodyMatch && (bodyMatch[1] || bodyMatch[2] || bodyMatch[3])) || "";
    action = isForward ? "FORWARD" : isReply ? "REPLY" : "SEND";
  } else {
    // MCP tool call path (existing logic)
    if (
      !toolName.includes("send_email") &&
      !toolName.includes("send_reply") &&
      !toolName.includes("forward_email")
    ) {
      process.exit(0);
    }

    const rawTo = toolInput.to || toolInput.recipient || "(unknown)";
    const toList = Array.isArray(rawTo) ? rawTo : [rawTo];
    toDisplay = toList.join(", ");
    subject = toolInput.subject || "(no subject)";
    body = toolInput.body || toolInput.message || "";
    action = toolName.includes("forward")
      ? "FORWARD"
      : toolName.includes("reply")
        ? "REPLY"
        : "SEND";
  }

  // Log details to stderr so the user sees them in the Claude Code UI
  process.stderr.write(`\n[InboxAPI Send Guard] ${action}\n`);
  process.stderr.write(`  To:      ${toDisplay}\n`);
  process.stderr.write(`  Subject: ${subject}\n`);
  if (body.length > 0) {
    const preview = body.length > 200 ? body.substring(0, 200) + "..." : body;
    process.stderr.write(`  Body:    ${preview}\n`);
  }
  process.stderr.write("\n");

  // Check for self-send (common AI agent mistake)
  const hasInboxApiRecipient = toDisplay.includes("@inboxapi.ai") || toDisplay.includes("@inboxapi.com");
  if (hasInboxApiRecipient) {
    process.stderr.write(
      `  [WARNING] Recipient is an @inboxapi address. Did you mean to send to an external address?\n\n`,
    );
  }

  // Check for empty body
  if (body.trim().length === 0) {
    process.stderr.write(`  [WARNING] Email body is empty.\n\n`);
  }

  // Allow by default — the user sees the preview and can deny via Claude Code's permission prompt
  process.exit(0);
}

main();
