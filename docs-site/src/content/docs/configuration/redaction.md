---
title: Redaction
description: Protect sensitive data in captured events
---

import { Aside } from '@astrojs/starlight/components';

OISP Sensor includes built-in redaction to protect sensitive information before it's stored or exported.

## Redaction Modes

### Safe Mode (Default)

Redacts common sensitive patterns while preserving observability:

```toml
[redaction]
mode = "safe"
```

Automatically redacts:
- API keys (OpenAI, Anthropic, etc.)
- Authorization headers
- Bearer tokens
- Emails
- Phone numbers
- Credit card numbers
- SSNs
- IP addresses in content

**Example:**
```json
// Before
{"content": "My email is user@example.com and key is sk-abc123xyz..."}

// After
{"content": "My email is [REDACTED:email] and key is [REDACTED:api_key]"}
```

### Full Mode

Captures everything without redaction:

```toml
[redaction]
mode = "full"
```

<Aside type="caution">
Only use `full` mode in secure environments where all data can be safely stored.
</Aside>

### Minimal Mode

Maximum redaction for sensitive environments:

```toml
[redaction]
mode = "minimal"
```

Redacts:
- Everything in `safe` mode
- All message content (replaced with length indicator)
- Tool inputs and outputs
- File paths

**Example:**
```json
// Before
{"content": "Please analyze this data: ..."}

// After  
{"content": "[CONTENT:35 chars]"}
```

## Redaction Patterns

Default patterns (Safe mode):

| Pattern | Example | Replacement |
|---------|---------|-------------|
| OpenAI API Key | `sk-abc123...` | `[REDACTED:api_key]` |
| Anthropic API Key | `sk-ant-...` | `[REDACTED:api_key]` |
| Bearer Token | `Bearer eyJ...` | `Authorization: [REDACTED]` |
| Email | `user@example.com` | `[REDACTED:email]` |
| Phone (US) | `+1-555-123-4567` | `[REDACTED:phone]` |
| Credit Card | `4111-1111-1111-1111` | `[REDACTED:cc]` |
| SSN | `123-45-6789` | `[REDACTED:ssn]` |

## Custom Redaction Rules

<Aside type="note">
Custom redaction rules are planned for a future release. Currently, use the predefined modes.
</Aside>

## What's Preserved

Even in `safe` mode, observability data is preserved:

- Event timestamps
- Process information (PID, name, path)
- Provider and model names
- Token counts and costs
- Latency measurements
- Status codes and errors
- Tool names (not inputs)

This allows you to:
- Track costs and usage
- Monitor latency and errors
- Build process trees
- Analyze provider distribution

Without exposing:
- Actual prompts or responses
- User data in messages
- API credentials
- Personal information

## Per-Field Redaction

Different event fields have different sensitivity:

| Field | Safe Mode | Minimal Mode |
|-------|-----------|--------------|
| `messages[].content` | Pattern-based | Full redaction |
| `tools[].input` | Pattern-based | Full redaction |
| `choices[].content` | Pattern-based | Full redaction |
| `request_id` | Preserved | Preserved |
| `model.id` | Preserved | Preserved |
| `usage.*` | Preserved | Preserved |
| `latency_ms` | Preserved | Preserved |
| `file.path` | Preserved | Redacted |

## API Key Detection

OISP Sensor recognizes API keys from many providers:

```
OpenAI:     sk-...
Anthropic:  sk-ant-...
Google:     AIza...
Cohere:     ...
AWS:        AKIA...
Azure:      ...
Mistral:    ...
```

Keys in headers, query parameters, and request bodies are detected and redacted.

## Testing Redaction

Verify redaction is working:

```bash
# Start with safe mode
oisp-sensor record --redaction safe --output /tmp/test.jsonl

# Make an API call
curl https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer sk-your-actual-key" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "test user@example.com"}]}'

# Check redaction
cat /tmp/test.jsonl | jq '.data.messages'
# Should show [REDACTED:...] not actual content
```

## Docker Security

When running in Docker, consider:

```yaml
services:
  oisp-sensor:
    image: oximy/oisp-sensor
    privileged: true
    environment:
      - OISP_REDACTION_MODE=safe
    volumes:
      # Don't mount sensitive host paths
      - ./events:/var/log/oisp
    # Restrict network if only local capture needed
    network_mode: host
```

## Compliance Considerations

For regulatory compliance:

| Requirement | Recommended Mode |
|-------------|------------------|
| GDPR | `minimal` or custom rules |
| HIPAA | `minimal` with audit logging |
| SOC 2 | `safe` or `minimal` |
| Development | `safe` |
| Local testing | `full` |

## Best Practices

1. **Default to safe**: Start with `safe` mode and only use `full` when necessary
2. **Test redaction**: Verify patterns are working before production
3. **Secure storage**: Even with redaction, treat event logs as sensitive
4. **Access control**: Limit who can access event data
5. **Audit trail**: Log access to event data if required by compliance

