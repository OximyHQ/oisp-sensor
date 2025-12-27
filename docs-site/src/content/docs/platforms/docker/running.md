---
title: Running with Docker
description: Run OISP Sensor container
---

# Running with Docker

Run OISP Sensor as a Docker container.

## Basic Usage

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  ghcr.io/oximyhq/oisp-sensor:latest
```

**Flags explained:**
- `--privileged` - Required for eBPF
- `--pid=host` - See host processes
- `--network=host` - Access host network (required for SSL capture)
- `-v /sys:/sys:ro` - Read kernel interfaces
- `-v /usr:/usr:ro` - Access OpenSSL libraries
- `-v /lib:/lib:ro` - Access system libraries

## With Persistent Logs

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  -v $(pwd)/logs:/var/log/oisp \
  ghcr.io/oximyhq/oisp-sensor:latest \
  record --output /var/log/oisp/events.jsonl
```

## With Web UI

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  ghcr.io/oximyhq/oisp-sensor:latest \
  record --web
```

Access Web UI at: http://localhost:7777

## With Custom Config

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  -v $(pwd)/config.toml:/etc/oisp/config.toml:ro \
  ghcr.io/oximyhq/oisp-sensor:latest \
  record --config /etc/oisp/config.toml
```

## Environment Variables

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  -e RUST_LOG=debug \
  -e OTLP_API_KEY=your-key \
  ghcr.io/oximyhq/oisp-sensor:latest
```

## Run in Background

```bash
docker run -d --name oisp-sensor \
  --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  -v $(pwd)/logs:/var/log/oisp \
  ghcr.io/oximyhq/oisp-sensor:latest \
  record --output /var/log/oisp/events.jsonl

# View logs
docker logs -f oisp-sensor

# Stop
docker stop oisp-sensor

# Remove
docker rm oisp-sensor
```

## Export to OTLP

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  -e OTLP_API_KEY=your-key \
  ghcr.io/oximyhq/oisp-sensor:latest \
  record --export otlp --otlp-endpoint http://collector:4317
```

## Troubleshooting

### Container exits immediately

Check logs:

```bash
docker logs oisp-sensor
```

Common issues:
- Kernel too old (< 4.18)
- BTF not available
- Missing `/sys/kernel/btf/vmlinux`

### No events captured

Verify host mode:

```bash
docker inspect oisp-sensor | grep NetworkMode
# Should show: "NetworkMode": "host"
```

### Permission denied

Ensure `--privileged` flag is set.

## Next Steps

- [Docker Compose](./compose) - Multi-container setup
- [Cookbooks](/cookbooks/overview/) - Example configurations
