---
title: CI/CD Integration
description: Integrate OISP Sensor into your CI/CD pipelines for AI activity testing and validation
---


Integrate OISP Sensor into your CI/CD pipelines to capture, validate, and test AI activity during builds and deployments.

## Overview

Use OISP Sensor in CI/CD to:
- **Test AI integrations** - Verify API calls work correctly
- **Validate responses** - Check model outputs meet requirements
- **Track usage** - Monitor token consumption in tests
- **Detect regressions** - Catch breaking changes in AI interactions
- **Audit compliance** - Ensure proper redaction and data handling

---

## Quick Start

### GitHub Actions

```yaml
name: Test AI Integration

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install OISP Sensor
        run: |
          curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh
          sudo oisp-sensor check

      - name: Start OISP Sensor
        run: |
          sudo oisp-sensor record --output /tmp/events.jsonl --no-ui &
          SENSOR_PID=$!
          echo "SENSOR_PID=$SENSOR_PID" >> $GITHUB_ENV
          sleep 2

      - name: Run tests
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          npm install
          npm test

      - name: Stop sensor
        if: always()
        run: |
          sudo kill $SENSOR_PID || true
          sleep 2

      - name: Validate events
        run: |
          # Check events were captured
          test -f /tmp/events.jsonl

          # Check for ai.request events
          grep -q '"event_type":"ai.request"' /tmp/events.jsonl

          # Check for ai.response events
          grep -q '"event_type":"ai.response"' /tmp/events.jsonl

          echo "✅ AI activity captured successfully"

      - name: Upload events
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: oisp-events
          path: /tmp/events.jsonl
```

---

## GitLab CI

```yaml
test_ai_integration:
  image: ubuntu:22.04

  before_script:
    - apt-get update
    - apt-get install -y curl jq
    - curl -fsSL https://sensor.oisp.dev/install.sh | sh

  script:
    # Start sensor
    - oisp-sensor record --output /tmp/events.jsonl --no-ui &
    - SENSOR_PID=$!
    - sleep 2

    # Run tests
    - npm install
    - npm test

    # Stop sensor
    - kill $SENSOR_PID || true
    - sleep 2

    # Validate
    - |
      if ! grep -q '"event_type":"ai.request"' /tmp/events.jsonl; then
        echo "❌ No ai.request events captured"
        exit 1
      fi
      echo "✅ AI activity validated"

  artifacts:
    paths:
      - /tmp/events.jsonl
    when: always
```

---

## Jenkins

```groovy
pipeline {
    agent any

    environment {
        OPENAI_API_KEY = credentials('openai-api-key')
    }

    stages {
        stage('Setup') {
            steps {
                sh 'curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh'
            }
        }

        stage('Test with OISP') {
            steps {
                script {
                    // Start sensor
                    sh 'sudo oisp-sensor record --output /tmp/events.jsonl --no-ui &'
                    env.SENSOR_PID = sh(script: 'echo $!', returnStdout: true).trim()
                    sleep 2

                    // Run tests
                    sh 'npm install && npm test'

                    // Stop sensor
                    sh "sudo kill ${env.SENSOR_PID} || true"
                    sleep 2

                    // Validate
                    sh '''
                        if ! grep -q '"event_type":"ai.request"' /tmp/events.jsonl; then
                            echo "❌ No AI events captured"
                            exit 1
                        fi
                        echo "✅ AI activity validated"
                    '''
                }
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: '/tmp/events.jsonl', allowEmptyArchive: true
        }
    }
}
```

---

## CircleCI

```yaml
version: 2.1

jobs:
  test:
    docker:
      - image: ubuntu:22.04

    steps:
      - checkout

      - run:
          name: Install dependencies
          command: |
            apt-get update
            apt-get install -y curl sudo jq

      - run:
          name: Install OISP Sensor
          command: curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh

      - run:
          name: Start sensor
          command: |
            sudo oisp-sensor record --output /tmp/events.jsonl --no-ui &
            echo $! > /tmp/sensor.pid
            sleep 2
          background: true

      - run:
          name: Run tests
          command: |
            npm install
            npm test

      - run:
          name: Stop sensor
          command: |
            sudo kill $(cat /tmp/sensor.pid) || true
            sleep 2
          when: always

      - run:
          name: Validate events
          command: |
            grep -q '"event_type":"ai.request"' /tmp/events.jsonl
            echo "✅ Events captured"

      - store_artifacts:
          path: /tmp/events.jsonl
          destination: oisp-events

workflows:
  version: 2
  test:
    jobs:
      - test
```

---

## Docker-Based CI

If your CI already uses Docker:

```yaml
# GitHub Actions with Docker
jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Run tests with OISP
        run: |
          docker run --rm \
            --privileged \
            --pid=host \
            -v $(pwd):/workspace \
            -w /workspace \
            -e OPENAI_API_KEY=${{ secrets.OPENAI_API_KEY }} \
            ghcr.io/oximyhq/oisp-sensor:latest \
            bash -c '
              oisp-sensor record --output /workspace/events.jsonl --no-ui &
              SENSOR_PID=$!
              sleep 2
              npm install && npm test
              kill $SENSOR_PID || true
              sleep 2
            '

      - name: Validate
        run: |
          grep -q '"event_type":"ai.request"' events.jsonl
          echo "✅ Events captured"
```

---

## Advanced Validation

### Validate Specific Providers

```bash
#!/bin/bash
# validate-providers.sh

EVENTS_FILE="/tmp/events.jsonl"

# Check OpenAI was used
if ! jq -r 'select(.data.provider == "OpenAI")' "$EVENTS_FILE" | grep -q .; then
    echo "❌ Expected OpenAI provider"
    exit 1
fi

echo "✅ OpenAI provider validated"
```

### Validate Models

```bash
#!/bin/bash
# validate-models.sh

EVENTS_FILE="/tmp/events.jsonl"

# Check gpt-4o-mini was used
if ! jq -r 'select(.data.model == "gpt-4o-mini")' "$EVENTS_FILE" | grep -q .; then
    echo "❌ Expected gpt-4o-mini model"
    exit 1
fi

echo "✅ Model validated"
```

### Validate Token Usage

```bash
#!/bin/bash
# validate-tokens.sh

EVENTS_FILE="/tmp/events.jsonl"

# Calculate total tokens
TOTAL_TOKENS=$(jq -r 'select(.event_type == "ai.response") | .data.usage.total_tokens' "$EVENTS_FILE" | awk '{sum+=$1} END {print sum}')

echo "Total tokens used: $TOTAL_TOKENS"

# Check threshold
MAX_TOKENS=10000
if [ "$TOTAL_TOKENS" -gt "$MAX_TOKENS" ]; then
    echo "❌ Token usage ($TOTAL_TOKENS) exceeds limit ($MAX_TOKENS)"
    exit 1
fi

echo "✅ Token usage within limits"
```

### Validate Response Content

```bash
#!/bin/bash
# validate-content.sh

EVENTS_FILE="/tmp/events.jsonl"

# Check response contains expected content
if ! jq -r 'select(.event_type == "ai.response") | .data.choices[0].message.content' "$EVENTS_FILE" | grep -q "expected phrase"; then
    echo "❌ Response doesn't contain expected content"
    exit 1
fi

echo "✅ Response content validated"
```

---

## Integration Testing

### Test Suite Example

```javascript
// tests/ai-integration.test.js
const fs = require('fs');
const { OpenAI } = require('openai');

describe('AI Integration', () => {
  let client;
  const eventsFile = '/tmp/events.jsonl';

  beforeAll(() => {
    client = new OpenAI();
    // Clear events file
    if (fs.existsSync(eventsFile)) {
      fs.unlinkSync(eventsFile);
    }
  });

  afterAll(async () => {
    // Wait for events to be written
    await new Promise(resolve => setTimeout(resolve, 2000));
  });

  test('should capture chat completion', async () => {
    const response = await client.chat.completions.create({
      model: 'gpt-4o-mini',
      messages: [{ role: 'user', content: 'Say hello!' }]
    });

    expect(response.choices[0].message.content).toBeTruthy();
  });

  test('events should be captured', () => {
    const events = fs.readFileSync(eventsFile, 'utf8')
      .split('\n')
      .filter(line => line.trim())
      .map(line => JSON.parse(line));

    const requestEvents = events.filter(e => e.event_type === 'ai.request');
    const responseEvents = events.filter(e => e.event_type === 'ai.response');

    expect(requestEvents.length).toBeGreaterThan(0);
    expect(responseEvents.length).toBeGreaterThan(0);
  });
});
```

---

## Performance Benchmarking

### Benchmark Token Usage

```yaml
# .github/workflows/benchmark.yml
name: AI Performance Benchmark

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Run benchmark with OISP
        run: |
          sudo oisp-sensor record --output /tmp/events.jsonl --no-ui &
          SENSOR_PID=$!
          sleep 2

          npm install
          npm run benchmark

          sudo kill $SENSOR_PID || true
          sleep 2

      - name: Analyze usage
        run: |
          # Calculate metrics
          TOTAL_TOKENS=$(jq -r 'select(.event_type == "ai.response") | .data.usage.total_tokens' /tmp/events.jsonl | awk '{sum+=$1} END {print sum}')
          REQUEST_COUNT=$(grep -c '"event_type":"ai.request"' /tmp/events.jsonl)

          echo "## Benchmark Results" >> $GITHUB_STEP_SUMMARY
          echo "- Total tokens: $TOTAL_TOKENS" >> $GITHUB_STEP_SUMMARY
          echo "- Requests: $REQUEST_COUNT" >> $GITHUB_STEP_SUMMARY
          echo "- Avg tokens/request: $((TOTAL_TOKENS / REQUEST_COUNT))" >> $GITHUB_STEP_SUMMARY

      - name: Compare with baseline
        run: |
          # Compare with previous run
          # Store baseline in repo or artifact storage
          # Fail if tokens increased by >10%
          ./scripts/compare-benchmark.sh
```

---

## Compliance Validation

### Validate Redaction

```bash
#!/bin/bash
# validate-redaction.sh

EVENTS_FILE="/tmp/events.jsonl"

# Check no API keys in events
if jq -r '.data.messages[].content' "$EVENTS_FILE" | grep -qE 'sk-[a-zA-Z0-9]{48}'; then
    echo "❌ Found API key in events (redaction failed)"
    exit 1
fi

# Check no email addresses
if jq -r '.data.messages[].content' "$EVENTS_FILE" | grep -qE '[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}'; then
    echo "❌ Found email address in events (redaction failed)"
    exit 1
fi

echo "✅ Redaction validated"
```

### Validate Data Retention

```bash
#!/bin/bash
# validate-retention.sh

EVENTS_FILE="/tmp/events.jsonl"

# Check events are not persisted beyond test run
if [ -f "/var/log/oisp/events.jsonl" ]; then
    echo "❌ Events persisted to disk (should be ephemeral in CI)"
    exit 1
fi

echo "✅ Ephemeral storage validated"
```

---

## Multi-Stage Pipelines

### Test → Staging → Production

```yaml
# .github/workflows/deploy.yml
name: Deploy with AI Validation

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Test with OISP
        run: ./scripts/ci-test-with-oisp.sh

  deploy-staging:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to staging
        run: ./scripts/deploy-staging.sh

      - name: Smoke test with OISP
        run: |
          sudo oisp-sensor record --output /tmp/events.jsonl --no-ui &
          SENSOR_PID=$!
          sleep 2

          # Run smoke tests
          npm run smoke-test:staging

          sudo kill $SENSOR_PID || true

          # Validate
          grep -q '"event_type":"ai.request"' /tmp/events.jsonl

  deploy-production:
    needs: deploy-staging
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to production
        run: ./scripts/deploy-production.sh
```

---

## Reusable Scripts

### `ci-test-with-oisp.sh`

```bash
#!/bin/bash
set -e

OUTPUT_FILE="${OUTPUT_FILE:-/tmp/events.jsonl}"
SENSOR_CONFIG="${SENSOR_CONFIG:-}"

echo "🚀 Starting OISP Sensor..."

# Start sensor
if [ -n "$SENSOR_CONFIG" ]; then
    sudo oisp-sensor record --output "$OUTPUT_FILE" --config "$SENSOR_CONFIG" --no-ui &
else
    sudo oisp-sensor record --output "$OUTPUT_FILE" --no-ui &
fi

SENSOR_PID=$!
echo "Sensor PID: $SENSOR_PID"
sleep 2

# Trap to ensure cleanup
cleanup() {
    echo "🛑 Stopping sensor..."
    sudo kill $SENSOR_PID || true
    sleep 2
}
trap cleanup EXIT

# Run tests
echo "🧪 Running tests..."
npm install
npm test

# Validate events
echo "✅ Validating events..."
if [ ! -f "$OUTPUT_FILE" ]; then
    echo "❌ Events file not found"
    exit 1
fi

REQUEST_COUNT=$(grep -c '"event_type":"ai.request"' "$OUTPUT_FILE" || echo 0)
RESPONSE_COUNT=$(grep -c '"event_type":"ai.response"' "$OUTPUT_FILE" || echo 0)

echo "Captured events:"
echo "  - Requests: $REQUEST_COUNT"
echo "  - Responses: $RESPONSE_COUNT"

if [ "$REQUEST_COUNT" -eq 0 ] || [ "$RESPONSE_COUNT" -eq 0 ]; then
    echo "❌ No AI events captured"
    exit 1
fi

echo "✅ CI validation complete"
```

Make executable:

```bash
chmod +x scripts/ci-test-with-oisp.sh
```

---

## Troubleshooting

### Sensor fails to start in CI

**Problem:** Permission denied or eBPF errors

**Solution:** Use Docker with `--privileged`:

```yaml
- name: Run with Docker
  run: |
    docker run --rm --privileged --pid=host \
      -v $(pwd):/workspace -w /workspace \
      ghcr.io/oximyhq/oisp-sensor:latest \
      bash -c 'oisp-sensor record --output events.jsonl --no-ui & ...'
```

### Events not captured

**Problem:** Sensor stops before events are written

**Solution:** Add delay before stopping:

```bash
npm test
sleep 3  # Wait for events to be written
kill $SENSOR_PID
```

### High CI runtime

**Problem:** Sensor adds overhead to CI

**Solution:** Only run on specific branches or with label:

```yaml
on:
  pull_request:
    types: [labeled]

jobs:
  test:
    if: contains(github.event.pull_request.labels.*.name, 'test-ai')
```

---

## Best Practices

1. **Ephemeral storage** - Use /tmp for CI, don't persist events
2. **Timeouts** - Set reasonable timeouts for sensor operations
3. **Cleanup** - Always stop sensor in `always` or `post` steps
4. **Artifacts** - Upload events as artifacts for debugging
5. **Validation** - Validate events before considering test successful
6. **Caching** - Don't cache sensor binaries (version changes)
7. **Secrets** - Use secrets manager for API keys, never hardcode

---

## Next Steps

- **[Multi-Node Deployment](./multi-node/)** - Production deployment patterns
- **[Cookbooks](/cookbooks/overview/)** - Example integrations to test
- **[Configuration](/configuration/config-file/)** - Advanced config options
