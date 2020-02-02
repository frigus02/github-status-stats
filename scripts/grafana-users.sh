#!/bin/bash
set -euo pipefail

GRAFANA_ADMIN_USERNAME=$(kubectl get secret ghss-grafana -o jsonpath='{.data.username}' | base64 --decode)
GRAFANA_ADMIN_PASSWORD=$(kubectl get secret ghss-grafana -o jsonpath='{.data.password}' | base64 --decode)

kubectl run grafana-users \
    --image curlimages/curl:7.68.0 \
    --rm \
    --restart=Never \
    --attach \
    --stdin \
    -- \
    curl \
    -sSi \
    -u "$GRAFANA_ADMIN_USERNAME:$GRAFANA_ADMIN_PASSWORD" \
    "http://ghss-grafana/api/users"
