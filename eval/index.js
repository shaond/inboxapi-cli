const { spawn, execFileSync } = require('child_process');
const readline = require('readline');
const Anthropic = require('@anthropic-ai/sdk');

const path = require('path');

const WORKTREE_DIR = path.join(__dirname, '..');
const DEFAULT_MODEL = 'claude-sonnet-4-5-latest';

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

const question = (query) => new Promise((resolve) => rl.question(query, resolve));

function runCommand(...args) {
    return execFileSync('cargo', ['run', '--', ...args], { cwd: WORKTREE_DIR, encoding: 'utf-8' });
}

class ProxyClient {
    constructor() {
        this._proc = null;
        this._lineReader = null;
        this._lineBuffer = [];
        this._lineWaiters = [];
        this._nextId = 1;
    }

    async start() {
        this._proc = spawn('cargo', ['run', '--', 'proxy'], {
            cwd: WORKTREE_DIR,
            stdio: ['pipe', 'pipe', 'inherit']
        });

        this._lineReader = readline.createInterface({ input: this._proc.stdout });
        this._lineReader.on('line', (line) => {
            const trimmed = line.trim();
            if (!trimmed) return;
            try {
                const parsed = JSON.parse(trimmed);
                if (this._lineWaiters.length > 0) {
                    const { resolve } = this._lineWaiters.shift();
                    resolve(parsed);
                } else {
                    this._lineBuffer.push(parsed);
                }
            } catch {
                // Ignore non-JSON output (e.g., cargo build messages)
            }
        });

        // Initialize
        this._send({
            jsonrpc: '2.0',
            id: this._nextId++,
            method: 'initialize',
            params: {
                protocolVersion: '2024-11-05',
                capabilities: {},
                clientInfo: { name: 'eval-script', version: '1.0.0' }
            }
        });

        const initResponse = await this._receive(10000);
        if (!initResponse.result) {
            throw new Error(`Initialize failed: ${JSON.stringify(initResponse)}`);
        }

        // Send initialized notification
        this._send({
            jsonrpc: '2.0',
            method: 'notifications/initialized'
        });

        return initResponse;
    }

    _send(message) {
        this._proc.stdin.write(JSON.stringify(message) + '\n');
    }

    _receive(timeoutMs = 30000) {
        return new Promise((resolve, reject) => {
            if (this._lineBuffer.length > 0) {
                resolve(this._lineBuffer.shift());
                return;
            }

            const timer = setTimeout(() => {
                const idx = this._lineWaiters.findIndex((w) => w.resolve === resolve);
                if (idx !== -1) this._lineWaiters.splice(idx, 1);
                reject(new Error(`Timeout after ${timeoutMs}ms waiting for response`));
            }, timeoutMs);

            this._lineWaiters.push({
                resolve: (value) => {
                    clearTimeout(timer);
                    resolve(value);
                }
            });
        });
    }

    async getTools() {
        this._send({
            jsonrpc: '2.0',
            id: this._nextId++,
            method: 'tools/list'
        });
        const response = await this._receive(10000);
        return response.result.tools;
    }

    async callTool(name, args) {
        this._send({
            jsonrpc: '2.0',
            id: this._nextId++,
            method: 'tools/call',
            params: { name, arguments: args }
        });
        return this._receive(30000);
    }

    close() {
        if (this._lineReader) this._lineReader.close();
        if (this._proc) this._proc.kill();
    }
}

async function main() {
    console.log('--- InboxAPI Agent Evaluation ---');

    // 1. Check Auth
    try {
        const whoami = JSON.parse(runCommand('whoami'));
        console.log(`Authenticated as: ${whoami.account_name} (${whoami.email})`);
    } catch {
        console.error('Error: Not authenticated. Run `cargo run -- login` first.');
        process.exit(1);
    }

    // 2. Get Addressbook
    const addressbookData = JSON.parse(runCommand('get-addressbook'));
    const contacts = addressbookData.addressbook;

    if (!contacts || contacts.length === 0) {
        console.error('Error: Addressbook is empty. Cannot run evaluations.');
        process.exit(1);
    }

    const maxContacts = Math.min(contacts.length, 5);
    console.log('\nRecent Contacts:');
    contacts.slice(0, maxContacts).forEach((c, i) => console.log(`${i + 1}. ${c.email}`));

    const choice = await question(`\nSelect a contact for integration tests (1-${maxContacts}): `);
    const choiceNum = parseInt(choice, 10);
    if (isNaN(choiceNum) || choiceNum < 1 || choiceNum > maxContacts) {
        console.error(`Invalid selection. Enter a number between 1 and ${maxContacts}.`);
        process.exit(1);
    }
    const testEmail = contacts[choiceNum - 1].email;
    console.log(`Using ${testEmail} for tests.`);

    // 3. Setup LLM
    if (!process.env.ANTHROPIC_API_KEY) {
        console.error('Error: ANTHROPIC_API_KEY environment variable is not set.');
        process.exit(1);
    }

    const anthropic = new Anthropic();
    const model = process.env.EVAL_MODEL || DEFAULT_MODEL;

    // 4. Start proxy
    const proxy = new ProxyClient();
    try {
        await proxy.start();
        const tools = await proxy.getTools();

        // Map MCP tools to Anthropic tool format
        const anthropicTools = tools.map((t) => ({
            name: t.name,
            description: t.description,
            input_schema: t.inputSchema
        }));

        const coreIntents = [
            {
                name: 'Send Email',
                expectedTool: 'send_email',
                prompt: `Send a short greeting email to ${testEmail} with subject "Integration Test" and body "Hello from the eval script!"`
            },
            {
                name: 'List Emails',
                expectedTool: 'get_emails',
                prompt: 'Get the last 3 emails from my inbox.'
            }
        ];

        for (const intent of coreIntents) {
            console.log(`\nEvaluating intent: ${intent.name}...`);

            const response = await anthropic.messages.create({
                model,
                max_tokens: 1024,
                tools: anthropicTools,
                messages: [{ role: 'user', content: intent.prompt }]
            });

            const textParts = response.content.filter((c) => c.type === 'text');
            if (textParts.length > 0) {
                console.log('LLM response:', textParts.map((c) => c.text).join('\n'));
            }

            const toolCalls = response.content.filter((c) => c.type === 'tool_use');
            if (toolCalls.length === 0) {
                console.log(`[FAIL] ${intent.name} - No tool call generated.`);
                continue;
            }

            const matchingCall = toolCalls.find((tc) => tc.name === intent.expectedTool);
            if (!matchingCall) {
                console.log(
                    `[FAIL] ${intent.name} - Expected tool ${intent.expectedTool}, got: ${toolCalls.map((tc) => tc.name).join(', ')}`
                );
                continue;
            }

            console.log(`Calling tool: ${matchingCall.name} with args:`, matchingCall.input);
            try {
                const toolResult = await proxy.callTool(matchingCall.name, matchingCall.input);
                if (toolResult.error || (toolResult.result && toolResult.result.isError)) {
                    throw new Error(`Tool returned error: ${JSON.stringify(toolResult.error || toolResult.result)}`);
                }
                console.log(`Tool ${matchingCall.name} result received.`);
                console.log(`[PASS] ${intent.name}`);
            } catch (e) {
                console.error(`[FAIL] ${intent.name} - Tool call failed: ${e.message}`);
            }
        }
    } finally {
        proxy.close();
    }

    rl.close();
}

main().catch(console.error);
