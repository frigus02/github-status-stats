#!/bin/sh
set -eu

if [ "$(git diff --stat)" != "" ]; then
    TAG="dev"
else
    TAG=$(git rev-parse HEAD)
fi

docker pull frigus02/github-status-stats-build

docker build --cache-from=frigus02/github-status-stats-build --target build -t frigus02/github-status-stats-build .
docker build --cache-from=frigus02/github-status-stats-build -t "frigus02/github-status-stats:$TAG" .

if [ "$TAG" != "dev" ]; then
    docker login -u frigus02 -p "$DOCKER_PASSWORD"
    docker push frigus02/github-status-stats-build
    docker push "frigus02/github-status-stats:$TAG"
fi
