#!/bin/bash
# Chapter 2 - Phase 2.2: AI Provider Testing
# Tests with OpenAI and Anthropic API endpoints
# Note: These tests use echo endpoints that don't require API keys

set -e

echo "=================================="
echo "  OISP eBPF Capture - AI Provider Tests"
echo "=================================="
echo ""

# Test 1: OpenAI API format (mock request to httpbin)
echo "[TEST 1] OpenAI-style request format"
echo "Expected: Should capture the typical OpenAI chat completion request structure"
echo ""
curl -s -X POST https://httpbin.org/post \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer sk-test-fake-key" \
    -d '{
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, how are you?"}
        ],
        "max_tokens": 100,
        "temperature": 0.7
    }' | jq -r '.json // .data // "Response received"' 2>/dev/null || echo "(response captured)"
echo ""
echo "[TEST 1] COMPLETE - Check for OpenAI request pattern in capture"
echo ""

# Test 2: Anthropic API format (mock request to httpbin)
echo "[TEST 2] Anthropic-style request format"
echo "Expected: Should capture the typical Anthropic messages request structure"
echo ""
curl -s -X POST https://httpbin.org/post \
    -H "Content-Type: application/json" \
    -H "x-api-key: sk-ant-test-fake-key" \
    -H "anthropic-version: 2023-06-01" \
    -d '{
        "model": "claude-3-opus-20240229",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": "Explain quantum computing in simple terms."}
        ]
    }' | jq -r '.json // .data // "Response received"' 2>/dev/null || echo "(response captured)"
echo ""
echo "[TEST 2] COMPLETE - Check for Anthropic request pattern in capture"
echo ""

# Test 3: Streaming SSE simulation (using chunked response)
echo "[TEST 3] SSE-style streaming response (chunked transfer)"
echo "Expected: Should see multiple SSL_read events for chunked response"
echo ""
curl -s "https://httpbin.org/stream/5" | head -5
echo ""
echo "[TEST 3] COMPLETE - Should see multiple read events for streaming"
echo ""

# Test 4: Large response (>4KB, should show truncation)
echo "[TEST 4] Large response test (tests 4KB buffer limit)"
echo "Expected: captured_len=4096, data_len > 4096"
echo ""
# Request 8KB of data
curl -s "https://httpbin.org/bytes/8192" -o /dev/null
echo "(8KB binary response fetched)"
echo ""
echo "[TEST 4] COMPLETE - Check if data was truncated in capture"
echo ""

# Test 5: Tool call simulation
echo "[TEST 5] Tool/Function call simulation"
echo "Expected: Should capture tool_calls structure in request"
echo ""
curl -s -X POST https://httpbin.org/post \
    -H "Content-Type: application/json" \
    -d '{
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "What is the weather in Tokyo?"}],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get current weather for a location",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string", "description": "City name"}
                        },
                        "required": ["location"]
                    }
                }
            }
        ],
        "tool_choice": "auto"
    }' | jq -r '.json.tools[0].function.name // "tool captured"' 2>/dev/null || echo "(response captured)"
echo ""
echo "[TEST 5] COMPLETE - Check for tools structure in capture"
echo ""

echo "=================================="
echo "  All AI provider tests completed!"
echo "=================================="
echo ""
echo "Review the eBPF capture output to verify:"
echo "  [x] OpenAI request structure captured (model, messages)"
echo "  [x] Anthropic request structure captured (x-api-key header)"
echo "  [x] Streaming responses generate multiple read events"
echo "  [x] Large responses show truncation (captured_len < data_len)"
echo "  [x] Tool/function calls captured correctly"
echo ""

