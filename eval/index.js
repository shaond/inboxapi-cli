const { spawn, execSync } = require('child_process');
const readline = require('readline');
const Anthropic = require('@anthropic-ai/sdk');

// Configuration
const CARGO_RUN = 'cargo run --';
const WORKTREE_DIR = '..'; // Assuming this is run from eval/

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

const question = (query) => new Promise((resolve) => rl.question(query, resolve));

async function runCommand(command) {
    return execSync(`${CARGO_RUN} ${command}`, { cwd: WORKTREE_DIR, encoding: 'utf-8' });
}

async function getTools() {
    return new Promise((resolve, reject) => {
        const proc = spawn('cargo', ['run', '--', 'proxy'], { cwd: WORKTREE_DIR });
        let output = '';
        
        proc.stdout.on('data', (data) => {
            const line = data.toString();
            try {
                const json = JSON.parse(line);
                if (json.result && json.result.tools) {
                    proc.kill();
                    resolve(json.result.tools);
                }
            } catch (e) {
                // Ignore non-json output (like cargo build messages)
            }
        });

        proc.stdin.write(JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            method: "initialize",
            params: {
                protocolVersion: "2024-11-05",
                capabilities: {},
                clientInfo: { name: "eval-script", version: "1.0.0" }
            }
        }) + '\n');

        proc.stdin.write(JSON.stringify({
            jsonrpc: "2.0",
            id: 2,
            method: "tools/list"
        }) + '\n');
        
        setTimeout(() => {
            proc.kill();
            reject(new Error('Timeout getting tools'));
        }, 10000);
    });
}

async function callTool(name, args) {
    return new Promise((resolve, reject) => {
        const proc = spawn('cargo', ['run', '--', 'proxy'], { cwd: WORKTREE_DIR });
        let responseReceived = false;

        proc.stdout.on('data', (data) => {
            const lines = data.toString().split('\n');
            for (const line of lines) {
                if (!line.trim()) continue;
                try {
                    const json = JSON.parse(line);
                    if (json.id === 3) {
                        responseReceived = true;
                        proc.kill();
                        resolve(json.result);
                    }
                } catch (e) {}
            }
        });

        proc.stdin.write(JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            method: "initialize",
            params: {
                protocolVersion: "2024-11-05",
                capabilities: {},
                clientInfo: { name: "eval-script", version: "1.0.0" }
            }
        }) + '\n');

        proc.stdin.write(JSON.stringify({
            jsonrpc: "2.0",
            id: 3,
            method: "tools/call",
            params: {
                name: name,
                arguments: args
            }
        }) + '\n');

        setTimeout(() => {
            if (!responseReceived) {
                proc.kill();
                reject(new Error(`Timeout calling tool ${name}`));
            }
        }, 30000);
    });
}

async function main() {
    console.log('--- InboxAPI Agent Evaluation ---');
    
    // 1. Check Auth
    try {
        const whoami = JSON.parse(await runCommand('whoami'));
        console.log(`Authenticated as: ${whoami.account_name} (${whoami.email})`);
    } catch (e) {
        console.error('Error: Not authenticated. Run `cargo run -- login` first.');
        process.exit(1);
    }

    // 2. Get Addressbook
    const addressbookData = JSON.parse(await runCommand('get-addressbook'));
    const contacts = addressbookData.addressbook;
    
    if (contacts.length === 0) {
        console.error('Error: Addressbook is empty. Cannot run evaluations.');
        process.exit(1);
    }

    console.log('\nRecent Contacts:');
    contacts.slice(0, 5).forEach((c, i) => console.log(`${i + 1}. ${c.email}`));
    
    const choice = await question('\nSelect a contact for integration tests (1-5): ');
    const testEmail = contacts[parseInt(choice) - 1].email;
    console.log(`Using ${testEmail} for tests.`);

    // 3. Setup LLM
    if (!process.env.ANTHROPIC_API_KEY) {
        console.error('Error: ANTHROPIC_API_KEY environment variable is not set.');
        process.exit(1);
    }

    const anthropic = new Anthropic();
    const tools = await getTools();
    
    // Map MCP tools to Anthropic tool format
    const anthropicTools = tools.map(t => ({
        name: t.name,
        description: t.description,
        input_schema: t.inputSchema
    }));

    const coreIntents = [
        {
            name: 'Send Email',
            prompt: `Send a short greeting email to ${testEmail} with subject "Integration Test" and body "Hello from the eval script!"`
        },
        {
            name: 'List Emails',
            prompt: 'Get the last 3 emails from my inbox.'
        }
    ];

    for (const intent of coreIntents) {
        console.log(`\nEvaluating intent: ${intent.name}...`);
        
        let messages = [{ role: 'user', content: intent.prompt }];
        
        const response = await anthropic.messages.create({
            model: 'claude-3-5-sonnet-20240620',
            max_tokens: 1024,
            tools: anthropicTools,
            messages: messages
        });

        console.log('Claude response:', response.content.filter(c => c.type === 'text').map(c => c.text).join('\n'));
        
        const toolCalls = response.content.filter(c => c.type === 'tool_use');
        if (toolCalls.length > 0) {
            for (const toolUse of toolCalls) {
                console.log(`Calling tool: ${toolUse.name} with args:`, toolUse.input);
                try {
                    const result = await callTool(toolUse.name, toolUse.input);
                    console.log(`Tool ${toolUse.name} result received.`);
                    // We could feed this back to Claude for a final confirmation, 
                    // but for "intent validation", seeing the tool call is usually enough.
                } catch (e) {
                    console.error(`Tool call failed: ${e.message}`);
                }
            }
            console.log(`[PASS] ${intent.name}`);
        } else {
            console.log(`[FAIL] ${intent.name} - No tool call generated.`);
        }
    }

    rl.close();
}

main().catch(console.error);
