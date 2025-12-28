#!/usr/bin/env python3
"""
Bridge script to feed mitmproxy captured traffic into OISP sensor.

Usage:
  1. Start oisp-sensor: ./target/release/oisp-sensor record --output events.jsonl
  2. Run mitmproxy with this addon: mitmproxy -s scripts/mitmproxy-bridge.py
  3. Configure your terminal to use proxy: export HTTPS_PROXY=http://127.0.0.1:8080
  4. Make AI API calls - they'll be captured by OISP!

AI domains are loaded from oisp-spec-bundle.json (same as the sensor).
"""

import json
import socket
import base64
import time
import os
import re
from pathlib import Path
from mitmproxy import http, ctx

SOCKET_PATH = "/tmp/oisp.sock"

# Load AI domains from spec bundle (same source as sensor)
def load_ai_domains():
    """Load AI domains from oisp-spec-bundle.json"""
    domains = set()
    patterns = []

    # Find spec bundle (check multiple locations)
    possible_paths = [
        Path(__file__).parent.parent / "crates" / "oisp-core" / "data" / "oisp-spec-bundle.json",
        Path.home() / ".cache" / "oisp" / "spec-bundle.json",
        Path("/usr/share/oisp/spec-bundle.json"),
    ]

    bundle_path = None
    for path in possible_paths:
        if path.exists():
            bundle_path = path
            break

    if not bundle_path:
        ctx.log.warn("oisp-spec-bundle.json not found, using fallback domains")
        return {
            "api.openai.com", "api.anthropic.com", "generativelanguage.googleapis.com",
            "api.cohere.ai", "api.mistral.ai", "api.groq.com", "api.together.xyz",
        }, []

    try:
        with open(bundle_path) as f:
            bundle = json.load(f)

        # Get exact domains from domain_index
        domains = set(bundle.get("domain_index", {}).keys())

        # Get patterns for wildcard matching
        for pattern in bundle.get("domain_patterns", []):
            try:
                patterns.append(re.compile(pattern["regex"], re.IGNORECASE))
            except re.error:
                pass

        ctx.log.info(f"Loaded {len(domains)} domains and {len(patterns)} patterns from {bundle_path}")
    except Exception as e:
        ctx.log.error(f"Failed to load spec bundle: {e}")

    return domains, patterns

AI_DOMAINS, AI_PATTERNS = load_ai_domains()


class OISPBridge:
    def __init__(self):
        self.sock = None
        self.connect()

    def connect(self):
        """Connect to OISP socket"""
        try:
            if os.path.exists(SOCKET_PATH):
                self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.sock.connect(SOCKET_PATH)
                ctx.log.info(f"Connected to OISP socket at {SOCKET_PATH}")
            else:
                ctx.log.warn(f"OISP socket not found at {SOCKET_PATH}. Start oisp-sensor first!")
                self.sock = None
        except Exception as e:
            ctx.log.error(f"Failed to connect to OISP: {e}")
            self.sock = None

    def send_event(self, kind: str, data: bytes, flow: http.HTTPFlow):
        """Send event to OISP socket"""
        if not self.sock:
            self.connect()
            if not self.sock:
                return

        event = {
            "id": flow.id[:32],
            "timestamp_ns": int(time.time() * 1_000_000_000),
            "kind": kind,
            "pid": os.getpid(),
            "tid": None,
            "data": base64.b64encode(data).decode("utf-8"),
            "metadata": {
                "comm": "mitmproxy",
                "exe": "/usr/local/bin/mitmproxy",
                "uid": os.getuid(),
                "fd": None,
                "ppid": os.getppid(),
            },
            "remote_host": flow.request.host,
            "remote_port": flow.request.port,
        }

        try:
            line = json.dumps(event) + "\n"
            self.sock.sendall(line.encode("utf-8"))
            ctx.log.info(f"Sent {kind} event for {flow.request.host}")
        except BrokenPipeError:
            ctx.log.warn("Socket disconnected, reconnecting...")
            self.sock = None
            self.connect()
        except Exception as e:
            ctx.log.error(f"Failed to send event: {e}")

    def is_ai_domain(self, host: str) -> bool:
        """Check if host is an AI provider (uses spec bundle)"""
        host_lower = host.lower()

        # Exact match
        if host_lower in AI_DOMAINS:
            return True

        # Pattern match (for wildcards like *.openai.azure.com)
        for pattern in AI_PATTERNS:
            if pattern.match(host_lower):
                return True

        return False


bridge = OISPBridge()


def request(flow: http.HTTPFlow):
    """Capture AI API requests"""
    if not bridge.is_ai_domain(flow.request.host):
        return

    # Reconstruct HTTP request
    request_line = f"{flow.request.method} {flow.request.path} HTTP/1.1\r\n"
    headers = "\r\n".join(f"{k}: {v}" for k, v in flow.request.headers.items())
    body = flow.request.content or b""

    http_data = f"{request_line}{headers}\r\n\r\n".encode("utf-8") + body
    bridge.send_event("SslWrite", http_data, flow)


def response(flow: http.HTTPFlow):
    """Capture AI API responses"""
    if not bridge.is_ai_domain(flow.request.host):
        return

    # Reconstruct HTTP response
    status_line = f"HTTP/1.1 {flow.response.status_code} {flow.response.reason}\r\n"
    headers = "\r\n".join(f"{k}: {v}" for k, v in flow.response.headers.items())
    body = flow.response.content or b""

    http_data = f"{status_line}{headers}\r\n\r\n".encode("utf-8") + body
    bridge.send_event("SslRead", http_data, flow)
