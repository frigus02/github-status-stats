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
docker build -t $BASE -f docker-base/Dockerfile .

$IMPORTER="$PREFIX-importer"
docker build --build-arg REGISTRY=localhost:5000 -t ${IMPORTER} -f importer/Dockerfile .

$WEBSITE="$PREFIX-website"
docker build --build-arg REGISTRY=localhost:5000 -t ${WEBSITE} -f website/Dockerfile .

docker push ${IMPORTER}
docker push ${WEBSITE}

# Deploy images
kustomize edit set image `
    frigus02/github-status-stats-importer="$(docker inspect --format '{{json .RepoDigests}}' ${IMPORTER} | jq -r '.[0]')" `
    frigus02/github-status-stats-website="$(docker inspect --format '{{json .RepoDigests}}' ${WEBSITE} | jq -r '.[0]')"

kustomize build | kubectl apply -f -
