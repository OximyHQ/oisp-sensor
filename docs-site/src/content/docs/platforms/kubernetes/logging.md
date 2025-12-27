---
title: Centralized Logging
description: Configure centralized log collection for Kubernetes deployments
---

# Centralized Logging

Aggregate events from all OISP Sensor pods across your cluster.

## Architecture Options

### Option 1: OTLP → OpenTelemetry Collector

**Best for:** OpenTelemetry-native environments

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ OISP Pod     │────▶│ OTLP         │────▶│ Backend      │
│ (Node 1)     │     │ Collector    │     │ (Grafana/    │
└──────────────┘     │              │     │  Datadog/    │
┌──────────────┐     │ (Aggregates  │     │  etc.)       │
│ OISP Pod     │────▶│  & Routes)   │     └──────────────┘
│ (Node 2)     │     └──────────────┘
└──────────────┘
```

### Option 2: Kafka

**Best for:** High-volume, streaming pipelines

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ OISP Pod     │────▶│ Kafka        │────▶│ Consumers    │
│ (Node 1)     │     │ Cluster      │     │ (Analytics/  │
└──────────────┘     │              │     │  Storage)    │
┌──────────────┐     │ (Durable     │     └──────────────┘
│ OISP Pod     │────▶│  Queue)      │
│ (Node 2)     │     └──────────────┘
└──────────────┘
```

### Option 3: Sidecar Collector

**Best for:** Per-pod processing

```
┌────────────────────────────┐
│  Pod                       │
│  ┌──────────┐ ┌──────────┐│
│  │ OISP     │→│ Fluent   ││────▶ Backend
│  │ Sensor   │ │ Bit      ││
│  └──────────┘ └──────────┘│
└────────────────────────────┘
```

---

## OTLP with OpenTelemetry Collector

### Deploy OTel Collector

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: otel-collector-config
  namespace: observability
data:
  config.yaml: |
    receivers:
      otlp:
        protocols:
          grpc:
            endpoint: 0.0.0.0:4317
          http:
            endpoint: 0.0.0.0:4318

    processors:
      batch:
        timeout: 10s
        send_batch_size: 100

      attributes:
        actions:
          - key: k8s.cluster.name
            value: production
            action: insert

    exporters:
      logging:
        loglevel: debug

      otlp/datadog:
        endpoint: api.datadoghq.com:443
        headers:
          DD-API-KEY: ${DATADOG_API_KEY}

      prometheus:
        endpoint: "0.0.0.0:8889"

      file:
        path: /var/log/otel/events.jsonl

    service:
      pipelines:
        traces:
          receivers: [otlp]
          processors: [batch, attributes]
          exporters: [logging, otlp/datadog, file]
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: otel-collector
  namespace: observability
spec:
  replicas: 2
  selector:
    matchLabels:
      app: otel-collector
  template:
    metadata:
      labels:
        app: otel-collector
    spec:
      containers:
        - name: otel-collector
          image: otel/opentelemetry-collector-contrib:latest
          args:
            - --config=/etc/otel/config.yaml
          ports:
            - containerPort: 4317  # OTLP gRPC
            - containerPort: 4318  # OTLP HTTP
            - containerPort: 8889  # Prometheus metrics
          env:
            - name: DATADOG_API_KEY
              valueFrom:
                secretKeyRef:
                  name: observability-secrets
                  key: datadog-api-key
          volumeMounts:
            - name: config
              mountPath: /etc/otel
      volumes:
        - name: config
          configMap:
            name: otel-collector-config
---
apiVersion: v1
kind: Service
metadata:
  name: otel-collector
  namespace: observability
spec:
  selector:
    app: otel-collector
  ports:
    - name: otlp-grpc
      port: 4317
      protocol: TCP
    - name: otlp-http
      port: 4318
      protocol: TCP
    - name: metrics
      port: 8889
      protocol: TCP
```

### Configure OISP Sensor

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: oisp-config
  namespace: oisp-system
data:
  config.toml: |
    [sensor]
    name = "k8s-${NODE_NAME}"

    [capture]
    ssl = true
    process = true

    [redaction]
    mode = "safe"

    [export.otlp]
    enabled = true
    endpoint = "http://otel-collector.observability:4317"
    headers = {
      "x-node-name" = "${NODE_NAME}",
      "x-cluster" = "production"
    }
```

Deploy:

```bash
kubectl apply -f otel-collector.yaml
kubectl apply -f oisp-sensor-daemonset.yaml
```

---

## Kafka

### Deploy Kafka (Strimzi Operator)

```bash
# Install Strimzi operator
kubectl create namespace kafka
kubectl apply -f 'https://strimzi.io/install/latest?namespace=kafka' -n kafka

# Deploy Kafka cluster
kubectl apply -f - <<EOF
apiVersion: kafka.strimzi.io/v1beta2
kind: Kafka
metadata:
  name: oisp-cluster
  namespace: kafka
spec:
  kafka:
    version: 3.6.0
    replicas: 3
    listeners:
      - name: plain
        port: 9092
        type: internal
        tls: false
    config:
      offsets.topic.replication.factor: 3
      transaction.state.log.replication.factor: 3
      transaction.state.log.min.isr: 2
    storage:
      type: ephemeral
  zookeeper:
    replicas: 3
    storage:
      type: ephemeral
EOF
```

### Configure OISP Sensor

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: oisp-config
  namespace: oisp-system
data:
  config.toml: |
    [sensor]
    name = "k8s-${NODE_NAME}"

    [capture]
    ssl = true
    process = true

    [redaction]
    mode = "safe"

    [export.kafka]
    enabled = true
    brokers = [
      "oisp-cluster-kafka-0.oisp-cluster-kafka-brokers.kafka:9092",
      "oisp-cluster-kafka-1.oisp-cluster-kafka-brokers.kafka:9092",
      "oisp-cluster-kafka-2.oisp-cluster-kafka-brokers.kafka:9092"
    ]
    topic = "oisp-events"
    compression = "snappy"
    batch_size = 100
```

### Create Kafka Topic

```bash
kubectl apply -f - <<EOF
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: oisp-events
  namespace: kafka
  labels:
    strimzi.io/cluster: oisp-cluster
spec:
  partitions: 12
  replicas: 3
  config:
    retention.ms: 604800000  # 7 days
    compression.type: snappy
EOF
```

### Consume Events

```bash
# Deploy Kafka consumer
kubectl run kafka-consumer -n kafka --rm -i --restart='Never' \
  --image=quay.io/strimzi/kafka:latest-kafka-3.6.0 -- \
  bin/kafka-console-consumer.sh \
  --bootstrap-server oisp-cluster-kafka-bootstrap:9092 \
  --topic oisp-events \
  --from-beginning
```

---

## Sidecar Pattern

For per-pod log processing:

```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: oisp-sensor
  namespace: oisp-system
spec:
  template:
    spec:
      containers:
        # Main sensor container
        - name: sensor
          image: ghcr.io/oximyhq/oisp-sensor:latest
          command:
            - /usr/bin/oisp-sensor
            - record
            - --output
            - /shared/events.jsonl
          volumeMounts:
            - name: shared-logs
              mountPath: /shared

        # Fluent Bit sidecar
        - name: fluent-bit
          image: fluent/fluent-bit:latest
          volumeMounts:
            - name: shared-logs
              mountPath: /shared
              readOnly: true
            - name: fluent-bit-config
              mountPath: /fluent-bit/etc
          command:
            - /fluent-bit/bin/fluent-bit
            - -c
            - /fluent-bit/etc/fluent-bit.conf

      volumes:
        - name: shared-logs
          emptyDir: {}
        - name: fluent-bit-config
          configMap:
            name: fluent-bit-config
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluent-bit-config
  namespace: oisp-system
data:
  fluent-bit.conf: |
    [SERVICE]
        Flush        5
        Daemon       Off
        Log_Level    info

    [INPUT]
        Name         tail
        Path         /shared/events.jsonl
        Parser       json

    [OUTPUT]
        Name         http
        Match        *
        Host         logs.example.com
        Port         443
        URI          /v1/logs
        tls          On
        Format       json
        Header       Authorization Bearer ${LOG_TOKEN}
```

---

## Storage Backends

### S3 Compatible

Use Fluent Bit S3 output:

```yaml
[OUTPUT]
    Name                         s3
    Match                        *
    bucket                       oisp-events
    region                       us-east-1
    store_dir                    /tmp/fluent-bit
    total_file_size              100M
    upload_timeout               1m
    use_put_object               On
    s3_key_format                /year=%Y/month=%m/day=%d/hour=%H/$UUID.json
```

### Elasticsearch

```yaml
[OUTPUT]
    Name            es
    Match           *
    Host            elasticsearch.logging
    Port            9200
    Index           oisp-events
    Type            _doc
    Logstash_Format On
    Logstash_Prefix oisp
```

### Loki

```yaml
[OUTPUT]
    Name            loki
    Match           *
    Host            loki.logging
    Port            3100
    Labels          job=oisp-sensor, cluster=production
```

---

## Monitoring Collector Health

### Prometheus Metrics

Expose OTel Collector metrics:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: otel-collector
  namespace: observability
spec:
  selector:
    matchLabels:
      app: otel-collector
  endpoints:
    - port: metrics
      interval: 30s
```

### Grafana Dashboard

Import dashboard for OTel Collector:
- Dashboard ID: 15983 (OpenTelemetry Collector)

### Alerts

```yaml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: otel-collector-alerts
  namespace: observability
spec:
  groups:
    - name: otel
      rules:
        - alert: OTelCollectorDown
          expr: up{job="otel-collector"} == 0
          for: 5m
          annotations:
            summary: "OTel Collector is down"
```

---

## Best Practices

1. **Use dedicated namespace** (`observability`, `logging`)
2. **Set resource limits** on collector pods
3. **Enable batching** for efficiency
4. **Use compression** (snappy for Kafka, gzip for S3)
5. **Monitor collector health** with Prometheus
6. **Set retention policies** (7-30 days typical)
7. **Use autoscaling** for collector deployments
8. **Secure secrets** with Kubernetes Secrets or external secret managers

---

## Next Steps

- [DaemonSet Deployment](./daemonset) - Deploy the sensor
- [Cookbooks](/cookbooks/kubernetes/daemonset/) - Full example
