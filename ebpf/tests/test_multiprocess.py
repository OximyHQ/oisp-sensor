#!/usr/bin/env python3
"""
Chapter 2 - Phase 2.3: Multi-Process Testing
Tests SSL capture with Python requests library.
Run while oisp-ebpf-capture is running.
"""

import requests
import json
import time
import os
import threading
from concurrent.futures import ThreadPoolExecutor

def print_header(title):
    print(f"\n{'='*50}")
    print(f"  {title}")
    print(f"{'='*50}\n")

def test_basic_get():
    """Test 1: Basic GET request with Python requests"""
    print("[TEST 1] Python requests - Basic GET")
    print(f"  PID: {os.getpid()}")
    
    resp = requests.get("https://httpbin.org/get", params={"source": "python", "test": "1"})
    print(f"  Status: {resp.status_code}")
    print(f"  Response size: {len(resp.content)} bytes")
    print("[TEST 1] COMPLETE\n")
    return resp.ok

def test_post_json():
    """Test 2: POST request with JSON body"""
    print("[TEST 2] Python requests - POST with JSON")
    print(f"  PID: {os.getpid()}")
    
    data = {
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello from Python test!"}
        ],
        "source": "oisp-multiprocess-test"
    }
    
    resp = requests.post(
        "https://httpbin.org/post",
        json=data,
        headers={"X-OISP-Test": "python-multiprocess"}
    )
    print(f"  Status: {resp.status_code}")
    print(f"  Echoed model: {resp.json().get('json', {}).get('model', 'N/A')}")
    print("[TEST 2] COMPLETE\n")
    return resp.ok

def test_session_keepalive():
    """Test 3: Multiple requests on same session (connection reuse)"""
    print("[TEST 3] Python requests - Session with connection reuse")
    print(f"  PID: {os.getpid()}")
    
    with requests.Session() as session:
        for i in range(3):
            resp = session.get(f"https://httpbin.org/get?request={i+1}")
            print(f"  Request {i+1}: {resp.status_code}")
            time.sleep(0.1)
    
    print("[TEST 3] COMPLETE - Check for same-connection events\n")
    return True

def test_concurrent_requests():
    """Test 4: Concurrent requests from multiple threads"""
    print("[TEST 4] Python requests - Concurrent threads")
    print(f"  Main PID: {os.getpid()}")
    
    def make_request(thread_id):
        tid = threading.current_thread().ident
        resp = requests.get(f"https://httpbin.org/get?thread={thread_id}")
        print(f"  Thread {thread_id} (TID ~{tid}): {resp.status_code}")
        return resp.ok
    
    with ThreadPoolExecutor(max_workers=3) as executor:
        results = list(executor.map(make_request, range(1, 4)))
    
    print(f"  All succeeded: {all(results)}")
    print("[TEST 4] COMPLETE - Check for different TID values\n")
    return all(results)

def test_large_response():
    """Test 5: Large response (tests chunking)"""
    print("[TEST 5] Python requests - Large response (10KB)")
    print(f"  PID: {os.getpid()}")
    
    resp = requests.get("https://httpbin.org/bytes/10240")
    print(f"  Status: {resp.status_code}")
    print(f"  Response size: {len(resp.content)} bytes")
    print("[TEST 5] COMPLETE - Should see multiple SSL_read events\n")
    return resp.ok

def test_streaming():
    """Test 6: Streaming response"""
    print("[TEST 6] Python requests - Streaming response")
    print(f"  PID: {os.getpid()}")
    
    resp = requests.get("https://httpbin.org/stream/5", stream=True)
    chunks = 0
    for chunk in resp.iter_lines():
        if chunk:
            chunks += 1
    
    print(f"  Status: {resp.status_code}")
    print(f"  Chunks received: {chunks}")
    print("[TEST 6] COMPLETE - Should see streaming read events\n")
    return resp.ok

def main():
    print_header("OISP eBPF Capture - Python Multi-Process Tests")
    print(f"Python PID: {os.getpid()}")
    print("Running tests with Python requests library...")
    
    tests = [
        ("Basic GET", test_basic_get),
        ("POST JSON", test_post_json),
        ("Session Keepalive", test_session_keepalive),
        ("Concurrent Threads", test_concurrent_requests),
        ("Large Response", test_large_response),
        ("Streaming", test_streaming),
    ]
    
    results = []
    for name, test_fn in tests:
        try:
            result = test_fn()
            results.append((name, result))
        except Exception as e:
            print(f"  ERROR: {e}")
            results.append((name, False))
    
    print_header("Test Results Summary")
    all_passed = True
    for name, passed in results:
        status = "PASS" if passed else "FAIL"
        print(f"  [{status}] {name}")
        all_passed = all_passed and passed
    
    print("\n" + "="*50)
    print("  Review eBPF capture output to verify:")
    print("  - comm field shows 'python3' or 'python'")
    print("  - PID matches the test PID above")
    print("  - TID varies for concurrent thread test")
    print("  - Large response shows multiple reads")
    print("="*50 + "\n")
    
    return 0 if all_passed else 1

if __name__ == "__main__":
    exit(main())

