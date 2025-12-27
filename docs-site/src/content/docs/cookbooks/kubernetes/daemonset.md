---
title: Kubernetes DaemonSet
description: Cluster-wide AI monitoring with OISP Sensor
---


Deploy OISP Sensor across your Kubernetes cluster.

## Overview

**What this demonstrates:**
- Kubernetes DaemonSet deployment
- Cluster-wide monitoring
- Multi-node event aggregation
- Centralized logging (OTLP/Kafka)

**Repository:** `oisp-cookbook/kubernetes/daemonset`

## Quick Deploy

```bash
cd oisp-cookbook/kubernetes/daemonset
kubectl apply -f manifests/
```

See [Kubernetes Platform Guide](/platforms/kubernetes/) for complete documentation.

## What's Included

- DaemonSet manifest
- ServiceAccount + RBAC
- ConfigMap for configuration
- Sample AI workload
- OTLP collector integration

## Use Cases

- Production Kubernetes clusters
- Multi-tenant environments
- Microservices AI monitoring
- Compliance and auditing
