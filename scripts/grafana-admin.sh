#!/bin/bash
set -euo pipefail

GRAFANA_ADMIN_USERNAME=$(kubectl get secret ghss-grafana -o jsonpath='{.data.username}' | base64 --decode)
GRAFANA_ADMIN_PASSWORD=$(kubectl get secret ghss-grafana -o jsonpath='{.data.password}' | base64 --decode)
GRAFANA_ORG="${1:?"org id as first arg required"}"
GRAFANA_USER="${2:?"user id as first arg required"}"
ACTION="${3:?"action as first arg required"}"

if [ "$ACTION" = "grant" ]; then
    BODY='{"role": "Admin"}'
elif [ "$ACTION" = "revoke" ]; then
    BODY='{"role": "Viewer"}'
else
    echo "action has to be either grant or revoke" >&2
    exit 1
fi

kubectl run grafana-admin \
    --image curlimages/curl:7.68.0 \
    --rm \
    --restart=Never \
    --attach \
    --stdin \
    -- \
    curl \
    -sSi \
    -X PATCH \
    -u "$GRAFANA_ADMIN_USERNAME:$GRAFANA_ADMIN_PASSWORD" \
    -H "Content-Type: application/json" \
    -d "$BODY" \
    "http://ghss-grafana/api/orgs/$GRAFANA_ORG/users/$GRAFANA_USER"
