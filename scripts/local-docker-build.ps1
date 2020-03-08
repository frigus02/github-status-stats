$ErrorActionPreference = "Stop"
function ExitIfNativeCallFailed($NativeCallSuccess)
{
    if (-not $NativeCallSuccess)
    {
        throw 'error making native call'
    }
}

# Make sure a local Docker registry is running
docker inspect registry | out-null
if (!$?)
{
    echo "Could not find local Docker registry. Starting one..."
    docker run -d -p 5000:5000 --restart always --name registry registry:2
}

# Build images
$PREFIX="localhost:5000/github-status-stats"

$BASE="$PREFIX-base"
docker build --build-arg CARGO_FLAGS= --build-arg CARGO_MODE=debug -t $BASE -f docker-base/Dockerfile .
ExitIfNativeCallFailed $?

$IMPORTER="$PREFIX-importer"
docker build --build-arg REGISTRY=localhost:5000 --build-arg CARGO_MODE=debug -t ${IMPORTER} -f importer/Dockerfile .
ExitIfNativeCallFailed $?

$WEBSITE="$PREFIX-website"
docker build --build-arg REGISTRY=localhost:5000 --build-arg CARGO_MODE=debug -t ${WEBSITE} -f website/Dockerfile .
ExitIfNativeCallFailed $?

docker push ${IMPORTER}
ExitIfNativeCallFailed $?
docker push ${WEBSITE}
ExitIfNativeCallFailed $?

# Deploy images
kustomize edit set image `
    frigus02/github-status-stats-importer="$(docker inspect --format '{{json .RepoDigests}}' ${IMPORTER} | jq -r '.[0]')" `
    frigus02/github-status-stats-website="$(docker inspect --format '{{json .RepoDigests}}' ${WEBSITE} | jq -r '.[0]')"
ExitIfNativeCallFailed $?

kustomize build | kubectl apply -f -
ExitIfNativeCallFailed $?
