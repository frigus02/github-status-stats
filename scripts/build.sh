#!/bin/bash
set -euo pipefail

PREFIX=frigus02/ghss
if [ "$(git diff --stat)" != "" ]; then
    TAG="dev"
else
    TAG=$(git rev-parse HEAD)
fi

BASE=$PREFIX-base
docker pull $BASE
docker build --cache-from=$BASE -t $BASE -f docker-base/Dockerfile .

IMPORTER=$PREFIX-importer
docker build --cache-from=$BASE -t "$IMPORTER:$TAG" -f crates/ghss_importer/Dockerfile .

WEBSITE=$PREFIX-website
docker build --cache-from=$BASE -t "$WEBSITE:$TAG" -f crates/ghss_website/Dockerfile .

if [ "$TAG" != "dev" ]; then
    docker login -u frigus02 -p "$DOCKER_PASSWORD"
    docker push $BASE
    docker push "$IMPORTER:$TAG"
    docker push "$WEBSITE:$TAG"
fi
