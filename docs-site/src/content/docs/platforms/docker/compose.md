---
title: Docker Compose
description: Run OISP Sensor with Docker Compose
---

# Docker Compose

Run OISP Sensor alongside your application containers.

## Basic docker-compose.yml

```yaml
version: '3.8'

services:
  oisp-sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - ./logs:/var/log/oisp
    command: record --output /var/log/oisp/events.jsonl
```

Run:

```bash
docker-compose up -d
docker-compose logs -f oisp-sensor
```

## With Application Container

```yaml
version: '3.8'

services:
  # Your application
  app:
    build: .
    ports:
      - "8000:8000"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}

  # OISP Sensor (monitoring the host)
  oisp-sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - ./logs:/var/log/oisp
    command: record --output /var/log/oisp/events.jsonl
    depends_on:
      - app
```

## With OTLP Export

```yaml
version: '3.8'

services:
  # OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector:latest
    ports:
      - "4317:4317"
    volumes:
      - ./otel-config.yaml:/etc/otel/config.yaml
    command: --config /etc/otel/config.yaml

  # OISP Sensor
  oisp-sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
    command: record --export otlp --otlp-endpoint http://localhost:4317
    depends_on:
      - otel-collector
```

## With Custom Config

```yaml
version: '3.8'

services:
  oisp-sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - ./config.toml:/etc/oisp/config.toml:ro
      - ./logs:/var/log/oisp
    command: record --config /etc/oisp/config.toml
```

**config.toml:**

```toml
[sensor]
name = "docker-sensor"

[capture]
ssl = true
process = true

[redaction]
mode = "safe"

[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
```

## Full Stack Example

```yaml
version: '3.8'

services:
  # Python app with OpenAI
  app:
    build: ./app
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    ports:
      - "8000:8000"

  # OISP Sensor
  sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - ./sensor-logs:/var/log/oisp
    command: record --output /var/log/oisp/events.jsonl

  # Viewer (optional - analyze events)
  viewer:
    build: ./viewer
    ports:
      - "3000:3000"
    volumes:
      - ./sensor-logs:/data:ro
    depends_on:
      - sensor
```

## Healthcheck

```yaml
services:
  oisp-sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - ./logs:/var/log/oisp
    command: record --web --output /var/log/oisp/events.jsonl
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:7777/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
```

## Complete Production Stack

```yaml
version: '3.8'

services:
  app:
    build: .
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    deploy:
      replicas: 3

  sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - ./config.toml:/etc/oisp/config.toml:ro
    environment:
      - KAFKA_BROKERS=kafka:9092
      - RUST_LOG=info
    command: record --config /etc/oisp/config.toml
    restart: unless-stopped

  kafka:
    image: confluentinc/cp-kafka:latest
    ports:
      - "9092:9092"

  kafka-ui:
    image: provectuslabs/kafka-ui:latest
    ports:
      - "8080:8080"
    environment:
      - KAFKA_CLUSTERS_0_NAME=local
      - KAFKA_CLUSTERS_0_BOOTSTRAPSERVERS=kafka:9092
```

## Next Steps

- [Overview](./overview) - Docker capabilities
- [Running](./running) - Run commands
- [Cookbooks](/cookbooks/overview/) - Full examples
