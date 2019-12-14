#!/bin/sh
set -eu

BUILD_IMAGE=frigus02/github-status-stats-build
FINAL_IMAGE=frigus02/github-status-stats
if [ "$(git diff --stat)" != "" ]; then
    TAG="dev"
else
    TAG=$(git rev-parse HEAD)
fi

docker pull $BUILD_IMAGE

docker build --cache-from=$BUILD_IMAGE --target build -t $BUILD_IMAGE .
docker build --cache-from=$BUILD_IMAGE -t "$FINAL_IMAGE:$TAG" .

if [ "$TAG" != "dev" ]; then
    docker login -u frigus02 -p "$DOCKER_PASSWORD"
    docker push $BUILD_IMAGE
    docker push "$FINAL_IMAGE:$TAG"
fi
