docker run `
    -it `
    --rm `
    -v $PWD/data:/data `
    --env-file .env `
    github-status-stats
