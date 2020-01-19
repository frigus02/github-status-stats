$PREFIX="frigus02/github-status-stats"

$BASE="$PREFIX-base"
docker build -t $BASE -f docker-base/Dockerfile .

$IMPORTER="$PREFIX-importer"
docker build -t $IMPORTER -f importer/Dockerfile .

$WEBSITE="$PREFIX-website"
docker build -t $WEBSITE -f website/Dockerfile .

docker push $IMPORTER
docker push $WEBSITE