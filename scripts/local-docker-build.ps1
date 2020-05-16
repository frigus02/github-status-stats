Param([Switch]$Release)

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
$PREFIX="localhost:5000/ghss"
$CARGO_FLAGS=""
$CARGO_MODE="debug"
if ($Release)
{
	$CARGO_FLAGS="--release"
	$CARGO_MODE="release"
}

$BASE="$PREFIX-base"
docker build --build-arg CARGO_FLAGS=$CARGO_FLAGS --build-arg CARGO_MODE=$CARGO_MODE -t $BASE -f docker-base/Dockerfile .
ExitIfNativeCallFailed $?

$IMPORTER="$PREFIX-importer"
docker build --build-arg REGISTRY=localhost:5000 --build-arg CARGO_MODE=$CARGO_MODE -t ${IMPORTER} -f crates/ghss_importer/Dockerfile .
ExitIfNativeCallFailed $?

$STORE="$PREFIX-store"
docker build --build-arg REGISTRY=localhost:5000 --build-arg CARGO_MODE=$CARGO_MODE -t ${STORE} -f crates/ghss_store/Dockerfile .
ExitIfNativeCallFailed $?

$WEBSITE="$PREFIX-website"
docker build --build-arg REGISTRY=localhost:5000 --build-arg CARGO_MODE=$CARGO_MODE -t ${WEBSITE} -f crates/ghss_website/Dockerfile .
ExitIfNativeCallFailed $?

docker push ${IMPORTER}
ExitIfNativeCallFailed $?
docker push ${STORE}
ExitIfNativeCallFailed $?
docker push ${WEBSITE}
ExitIfNativeCallFailed $?

# Deploy images
kustomize edit set image `
    frigus02/ghss-importer="$(docker inspect --format '{{json .RepoDigests}}' ${IMPORTER} | jq -r '.[0]')" `
    frigus02/ghss-store="$(docker inspect --format '{{json .RepoDigests}}' ${STORE} | jq -r '.[0]')" `
    frigus02/ghss-website="$(docker inspect --format '{{json .RepoDigests}}' ${WEBSITE} | jq -r '.[0]')"
ExitIfNativeCallFailed $?

kustomize build | kubectl apply -f -
ExitIfNativeCallFailed $?
