#!/usr/bin/env node
/**
 * Chapter 2 - Phase 2.3: Multi-Process Testing
 * Tests SSL capture with Node.js fetch/https.
 * Run while oisp-ebpf-capture is running.
 */

const https = require('https');

function printHeader(title) {
    console.log(`\n${'='.repeat(50)}`);
    console.log(`  ${title}`);
    console.log(`${'='.repeat(50)}\n`);
}

async function fetchJson(url, options = {}) {
    return new Promise((resolve, reject) => {
        const urlObj = new URL(url);
        const reqOptions = {
            hostname: urlObj.hostname,
            path: urlObj.pathname + urlObj.search,
            method: options.method || 'GET',
            headers: options.headers || {},
        };

        const req = https.request(reqOptions, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                resolve({
                    ok: res.statusCode >= 200 && res.statusCode < 300,
                    status: res.statusCode,
                    text: () => Promise.resolve(data),
                    json: () => Promise.resolve(JSON.parse(data)),
                });
            });
        });

        req.on('error', reject);

        if (options.body) {
            req.write(options.body);
        }
        req.end();
    });
}

async function testBasicGet() {
    console.log('[TEST 1] Node.js https - Basic GET');
    console.log(`  PID: ${process.pid}`);

    const resp = await fetchJson('https://httpbin.org/get?source=nodejs&test=1');
    console.log(`  Status: ${resp.status}`);
    const text = await resp.text();
    console.log(`  Response size: ${text.length} bytes`);
    console.log('[TEST 1] COMPLETE\n');
    return resp.ok;
}

async function testPostJson() {
    console.log('[TEST 2] Node.js https - POST with JSON');
    console.log(`  PID: ${process.pid}`);

    const data = JSON.stringify({
        model: 'gpt-4',
        messages: [
            { role: 'user', content: 'Hello from Node.js test!' }
        ],
        source: 'oisp-nodejs-test'
    });

    const resp = await fetchJson('https://httpbin.org/post', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-OISP-Test': 'nodejs-multiprocess',
        },
        body: data,
    });

    console.log(`  Status: ${resp.status}`);
    try {
        const json = await resp.json();
        console.log(`  Echoed model: ${json.json?.model || 'N/A'}`);
    } catch (e) {
        console.log('  (Could not parse JSON response)');
    }
    console.log('[TEST 2] COMPLETE\n');
    return resp.ok;
}

async function testSequentialRequests() {
    console.log('[TEST 3] Node.js https - Sequential requests');
    console.log(`  PID: ${process.pid}`);

    for (let i = 1; i <= 3; i++) {
        const resp = await fetchJson(`https://httpbin.org/get?request=${i}`);
        console.log(`  Request ${i}: ${resp.status}`);
        await new Promise(r => setTimeout(r, 100));
    }

    console.log('[TEST 3] COMPLETE\n');
    return true;
}

async function testConcurrentRequests() {
    console.log('[TEST 4] Node.js https - Concurrent requests');
    console.log(`  PID: ${process.pid}`);

    const promises = [1, 2, 3].map(async (i) => {
        const resp = await fetchJson(`https://httpbin.org/get?concurrent=${i}`);
        console.log(`  Request ${i}: ${resp.status}`);
        return resp.ok;
    });

    const results = await Promise.all(promises);
    console.log(`  All succeeded: ${results.every(r => r)}`);
    console.log('[TEST 4] COMPLETE\n');
    return results.every(r => r);
}

async function testOpenAIFormat() {
    console.log('[TEST 5] Node.js - OpenAI API format');
    console.log(`  PID: ${process.pid}`);

    const data = JSON.stringify({
        model: 'gpt-4-turbo',
        messages: [
            { role: 'system', content: 'You are a helpful coding assistant.' },
            { role: 'user', content: 'Write a hello world in Rust.' }
        ],
        stream: false,
        max_tokens: 500
    });

    const resp = await fetchJson('https://httpbin.org/post', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': 'Bearer sk-nodejs-test-key',
        },
        body: data,
    });

    console.log(`  Status: ${resp.status}`);
    console.log('[TEST 5] COMPLETE\n');
    return resp.ok;
}

async function main() {
    printHeader('OISP eBPF Capture - Node.js Multi-Process Tests');
    console.log(`Node.js PID: ${process.pid}`);
    console.log(`Node.js version: ${process.version}`);
    console.log('Running tests with Node.js https module...\n');

    const tests = [
        { name: 'Basic GET', fn: testBasicGet },
        { name: 'POST JSON', fn: testPostJson },
        { name: 'Sequential Requests', fn: testSequentialRequests },
        { name: 'Concurrent Requests', fn: testConcurrentRequests },
        { name: 'OpenAI Format', fn: testOpenAIFormat },
    ];

    const results = [];
    for (const { name, fn } of tests) {
        try {
            const result = await fn();
            results.push({ name, passed: result });
        } catch (e) {
            console.log(`  ERROR: ${e.message}`);
            results.push({ name, passed: false });
        }
    }

    printHeader('Test Results Summary');
    let allPassed = true;
    for (const { name, passed } of results) {
        const status = passed ? 'PASS' : 'FAIL';
        console.log(`  [${status}] ${name}`);
        allPassed = allPassed && passed;
    }

    console.log('\n' + '='.repeat(50));
    console.log('  Review eBPF capture output to verify:');
    console.log("  - comm field shows 'node'");
    console.log('  - PID matches the test PID above');
    console.log('  - JSON payloads are captured correctly');
    console.log('='.repeat(50) + '\n');

    process.exit(allPassed ? 0 : 1);
}

main();

