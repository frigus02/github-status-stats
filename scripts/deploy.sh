#!/bin/sh
set -eu

KUBECTL_VERSION=1.17.0
KUSTOMIZE_VERSION=3.5.4

mkdir -p "$HOME/bin"
PATH="$PATH:$HOME/bin"
curl -sfL \
    -o "$HOME/bin/kubectl" \
    https://storage.googleapis.com/kubernetes-release/release/v$KUBECTL_VERSION/bin/linux/amd64/kubectl
chmod +x "$HOME/bin/kubectl"
curl -sfL \
    -o "$HOME/bin/kustomize.tar.gz" \
    https://github.com/kubernetes-sigs/kustomize/releases/download/kustomize/v$KUSTOMIZE_VERSION/kustomize_v${KUSTOMIZE_VERSION}_linux_amd64.tar.gz
tar -xzf "$HOME/bin/kustomize.tar.gz"
rm "$HOME/bin/kustomize.tar.gz"
chmod +x "$HOME/bin/kustomize"

mkdir -p "$HOME/.kube"
echo "$KUBE_CONFIG" >"$HOME/.kube/config"

kustomize edit set image \
    "$(docker inspect --format '{{json .RepoDigests}}' "$IMPORTER:$TAG" | jq -r '.[0]')" \
    "$(docker inspect --format '{{json .RepoDigests}}' "$WEBSITE:$TAG" | jq -r '.[0]')"

# kustomize build | kubectl apply -f -
kustomize build
