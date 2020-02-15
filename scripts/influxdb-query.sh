#!/bin/bash
set -euo pipefail

INFLUXDB_ADMIN_USERNAME=$(kubectl get secret ghss-influxdb -o jsonpath='{.data.admin_username}' | base64 --decode)
INFLUXDB_ADMIN_PASSWORD=$(kubectl get secret ghss-influxdb -o jsonpath='{.data.admin_password}' | base64 --decode)
INFLUXDB_DB="r${1:?"repository id as first arg required"}"
INFLUXDB_QUERY="${2:?"query as second arg required"}"

kubectl run influxdb-query \
    --image curlimages/curl:7.68.0 \
    --rm \
    --restart=Never \
    --attach \
    --stdin \
    -- \
    curl \
    -i \
    -X POST \
    -u "$INFLUXDB_ADMIN_USERNAME:$INFLUXDB_ADMIN_PASSWORD" \
    --data-urlencode "q=$INFLUXDB_QUERY" \
    "http://ghss-influxdb/query?db=$INFLUXDB_DB"
