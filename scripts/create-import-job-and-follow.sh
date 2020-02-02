#!/bin/bash
set -euo pipefail

JOB_NAME="ghss-importer-$(date +%s)"
kubectl create job "$JOB_NAME" --from cronjob/ghss-importer

JOB_POD=$(kubectl get pod -l "job-name=$JOB_NAME,group=github-status-stats" -o name)
kubectl wait --for=condition=Ready "$JOB_POD"
kubectl logs -f "$JOB_POD"
