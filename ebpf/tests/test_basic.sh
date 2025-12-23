#!/bin/bash
# Chapter 2 - Phase 2.1: Basic HTTPS verification tests
# Run these tests while oisp-ebpf-capture is running

set -e

echo "=================================="
echo "  OISP eBPF Capture - Basic Tests"
echo "=================================="
echo ""

# Test 1: Basic HTTPS GET request
echo "[TEST 1] curl https://httpbin.org/get"
echo "Expected: Should see SSL_write with GET request, SSL_read with JSON response"
echo ""
curl -s https://httpbin.org/get | head -5
echo ""
echo "[TEST 1] COMPLETE - Check eBPF capture output for events"
echo ""

# Test 2: HTTPS POST request with body
echo "[TEST 2] curl https://httpbin.org/post with JSON body"
echo "Expected: Should see SSL_write with POST + JSON body, SSL_read with response"
echo ""
curl -s -X POST https://httpbin.org/post \
    -H "Content-Type: application/json" \
    -d '{"message": "Hello from OISP test", "test_id": 12345}' | head -10
echo ""
echo "[TEST 2] COMPLETE - Check eBPF capture output for events"
echo ""

# Test 3: Multiple sequential requests
echo "[TEST 3] Multiple sequential requests"
echo "Expected: Should see multiple SSL_write/SSL_read pairs"
echo ""
for i in 1 2 3; do
    echo "  Request $i..."
    curl -s "https://httpbin.org/get?request=$i" > /dev/null
done
echo ""
echo "[TEST 3] COMPLETE - Should see 3 request/response pairs"
echo ""

# Test 4: Headers inspection
echo "[TEST 4] Request with custom headers"
echo "Expected: Should see custom headers in SSL_write capture"
echo ""
curl -s https://httpbin.org/headers \
    -H "X-OISP-Test: chapter2-phase2.1" \
    -H "X-Custom-Header: testing-ebpf-capture" | head -15
echo ""
echo "[TEST 4] COMPLETE"
echo ""

echo "=================================="
echo "  All basic tests completed!"
echo "=================================="
echo ""
echo "Review the eBPF capture output to verify:"
echo "  [x] SSL_write captures show HTTP requests"
echo "  [x] SSL_read captures show HTTP responses"
echo "  [x] Data is plaintext (not encrypted)"
echo "  [x] PID/comm fields identify curl process"
echo ""

