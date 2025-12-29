# OISP Event Fixtures

This directory contains JSONL fixture files for testing and development. These fixtures allow you to develop the dashboard and TUI without needing live capture capabilities.

## Usage

```bash
# Replay events from a fixture file
oisp-sensor replay --input fixtures/demo-session.jsonl

# Instant replay (no timing delays)
oisp-sensor replay --input fixtures/demo-session.jsonl --speed 0

# 2x speed
oisp-sensor replay --input fixtures/demo-session.jsonl --speed 2

# Loop continuously
oisp-sensor replay --input fixtures/demo-session.jsonl --loop-playback

# With TUI instead of web
oisp-sensor replay --input fixtures/demo-session.jsonl --tui
```

## Directory Structure

```
fixtures/
├── README.md                    # This file
├── demo-session.jsonl           # Quick demo (curated subset for demos)
│
├── providers/                   # Provider-specific fixtures
│   ├── openai/
│   │   ├── chat-completion.jsonl
│   │   ├── streaming.jsonl
│   │   ├── function-calling.jsonl
│   │   ├── vision.jsonl
│   │   └── embeddings.jsonl
│   ├── anthropic/
│   │   ├── messages.jsonl
│   │   ├── streaming.jsonl
│   │   └── tool-use.jsonl
│   ├── google/
│   │   └── gemini.jsonl
│   ├── azure/
│   │   └── openai.jsonl
│   └── local/
│       └── ollama.jsonl
│
├── scenarios/                   # Multi-event scenario fixtures
│   ├── multi-turn-conversation.jsonl
│   ├── agent-tool-loop.jsonl
│   ├── concurrent-requests.jsonl
│   └── high-volume.jsonl
│
├── errors/                      # Error case fixtures
│   ├── rate-limit.jsonl
│   ├── auth-failure.jsonl
│   ├── timeout.jsonl
│   └── malformed-response.jsonl
│
└── edge-cases/                  # Edge case fixtures
    ├── large-context.jsonl
    ├── binary-content.jsonl
    ├── unicode-heavy.jsonl
    └── streaming-interrupted.jsonl
```

## File Format

Each fixture file contains newline-delimited JSON (JSONL) where each line is a valid OISP event:

```jsonl
{"oisp_version":"0.1","event_id":"01HQ...","event_type":"ai.request","ts":"2024-01-15T12:00:00Z",...}
{"oisp_version":"0.1","event_id":"01HQ...","event_type":"ai.response","ts":"2024-01-15T12:00:01Z",...}
```

## Creating New Fixtures

1. **From live capture**: Record real events and save them:
   ```bash
   oisp-sensor record --output my-events.jsonl
   ```

2. **From spec examples**: Use the examples in `oisp-spec/examples/`:
   ```bash
   cat ../oisp-spec/examples/ai-request-openai.json | jq -c . >> fixtures/providers/openai/chat-completion.jsonl
   ```

3. **Manual creation**: Create events following the OISP spec schema.

## Required Fields

Each event must have:
- `oisp_version`: Always "0.1"
- `event_id`: Unique ULID
- `event_type`: Event type (e.g., "ai.request", "ai.response")
- `ts`: ISO 8601 timestamp
- `source`: Collector info
- `confidence`: Confidence metadata
- `data`: Event-specific payload

## Notes

- Comments (lines starting with `#`) are supported and ignored during replay
- Empty lines are skipped
- Timestamps are used for replay timing when speed > 0
- Events are replayed in file order
