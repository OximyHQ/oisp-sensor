---
title: DaemonSet Deployment
description: Deploy OISP Sensor as a Kubernetes DaemonSet
---

# DaemonSet Deployment

Deploy OISP Sensor to monitor all nodes in your Kubernetes cluster.

## Quick Deploy

```bash
kubectl apply -f https://sensor.oisp.dev/manifests/daemonset.yaml
```

## Complete DaemonSet Manifest

Create `oisp-sensor-daemonset.yaml`:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: oisp-system
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: oisp-sensor
  namespace: oisp-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: oisp-sensor
rules:
  - apiGroups: [""]
    resources: ["pods", "nodes"]
    verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: oisp-sensor
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: oisp-sensor
subjects:
  - kind: ServiceAccount
    name: oisp-sensor
    namespace: oisp-system
---
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
    file = true
    network = true

    [redaction]
    mode = "safe"

    [export.jsonl]
    enabled = true
    path = "/var/log/oisp/events.jsonl"
    append = true
    rotate = true
    max_size_mb = 100
---
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: oisp-sensor
  namespace: oisp-system
  labels:
    app: oisp-sensor
spec:
  selector:
    matchLabels:
      app: oisp-sensor
  template:
    metadata:
      labels:
        app: oisp-sensor
    spec:
      serviceAccountName: oisp-sensor
      hostNetwork: true
      hostPID: true
      containers:
        - name: sensor
          image: ghcr.io/oximyhq/oisp-sensor:latest
          imagePullPolicy: Always
          command:
            - /usr/bin/oisp-sensor
            - record
            - --config
            - /etc/oisp/config.toml
          env:
            - name: NODE_NAME
              valueFrom:
                fieldRef:
                  fieldPath: spec.nodeName
            - name: POD_NAME
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
            - name: POD_NAMESPACE
              valueFrom:
                fieldRef:
                  fieldPath: metadata.namespace
            - name: RUST_LOG
              value: "info"
          securityContext:
            privileged: true
            capabilities:
              add:
                - SYS_ADMIN
                - BPF
                - PERFMON
                - NET_ADMIN
          resources:
            requests:
              cpu: 100m
              memory: 256Mi
            limits:
              cpu: 500m
              memory: 512Mi
          volumeMounts:
            - name: sys
              mountPath: /sys
              readOnly: true
            - name: usr
              mountPath: /usr
              readOnly: true
            - name: lib
              mountPath: /lib
              readOnly: true
            - name: config
              mountPath: /etc/oisp
              readOnly: true
            - name: logs
              mountPath: /var/log/oisp
      volumes:
        - name: sys
          hostPath:
            path: /sys
        - name: usr
          hostPath:
            path: /usr
        - name: lib
          hostPath:
            path: /lib
        - name: config
          configMap:
            name: oisp-config
        - name: logs
          hostPath:
            path: /var/log/oisp
            type: DirectoryOrCreate
      tolerations:
        # Run on all nodes including control plane
        - operator: Exists
```

Deploy:

```bash
kubectl apply -f oisp-sensor-daemonset.yaml
```

## Verify Deployment

```bash
# Check DaemonSet status
kubectl get daemonset -n oisp-system

# Check pods (should have one per node)
kubectl get pods -n oisp-system -o wide

# View logs from a specific pod
kubectl logs -n oisp-system -l app=oisp-sensor --tail=100

# Follow logs
kubectl logs -n oisp-system -l app=oisp-sensor -f
```

## With OTLP Export

Update ConfigMap to export to OpenTelemetry Collector:

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
    headers = { "x-node-name" = "${NODE_NAME}" }
```

## With Kafka Export

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
    brokers = ["kafka-0.kafka:9092", "kafka-1.kafka:9092"]
    topic = "oisp-events"
    compression = "snappy"
```

## Node Selector

To run only on specific nodes:

```yaml
spec:
  template:
    spec:
      nodeSelector:
        oisp-sensor: "enabled"
```

Then label nodes:

```bash
kubectl label nodes node-1 oisp-sensor=enabled
kubectl label nodes node-2 oisp-sensor=enabled
```

## Resource Limits

Adjust based on cluster size and activity:

```yaml
resources:
  requests:
    cpu: 200m        # For high-activity nodes
    memory: 512Mi
  limits:
    cpu: 1000m       # Allow bursts
    memory: 1Gi
```

## Pod Security Standards

For restricted environments:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: oisp-system
  labels:
    pod-security.kubernetes.io/enforce: privileged
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
```

## Update Strategy

Rolling update (default):

```yaml
spec:
  updateStrategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
```

Or immediate update (use with caution):

```yaml
spec:
  updateStrategy:
    type: OnDelete
```

## Troubleshooting

### Pods not starting

```bash
# Check pod status
kubectl describe pod -n oisp-system -l app=oisp-sensor

# Check events
kubectl get events -n oisp-system --sort-by='.lastTimestamp'
```

Common issues:
- Node kernel < 5.8
- BTF not available
- Privileged pod security not allowed

### No events captured

```bash
# Check logs
kubectl logs -n oisp-system -l app=oisp-sensor --tail=50

# Check if eBPF programs loaded
kubectl exec -n oisp-system -it <pod-name> -- bpftool prog list
```

### High resource usage

```bash
# Check resource usage
kubectl top pod -n oisp-system

# Increase limits if needed
kubectl edit daemonset -n oisp-system oisp-sensor
```

## Uninstall

```bash
kubectl delete -f oisp-sensor-daemonset.yaml

# Or
kubectl delete namespace oisp-system
```

## Next Steps

- [Centralized Logging](./logging) - Configure log aggregation
- [Cookbooks](/cookbooks/kubernetes/daemonset/) - Full example with app
