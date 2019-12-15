#!/bin/sh
set -eu
TAG=$(git rev-parse HEAD)
docker run \
    -it \
    --rm \
    -v "$PWD/data":/data \
    --env-file .env \
    "frigus02/github-status-stats:$TAG"
