if ($(git diff --stat))
{
    $TAG = "dev"
}
else
{
    $TAG = $(git rev-parse HEAD)
}

docker pull frigus02/github-status-stats-build

docker build --cache-from=frigus02/github-status-stats-build --target build -t frigus02/github-status-stats-build .
docker build --cache-from=frigus02/github-status-stats-build -t frigus02/github-status-stats:$TAG .

if ($TAG -ne "dev")
{
    docker push frigus02/github-status-stats-build
    docker push frigus02/github-status-stats:$TAG
}
