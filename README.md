# GitHub Status Stats

If you're using any CI system on your GitHub repository, chances are it pushes commit statuses to GitHub. The most recent ones are shown on the commit, as a :heavy_check_mark: tick or :x: cross.

GitHub stores a history of all commit statuses. If you retry a build on the same commit, it doesn't overwrite the previous status. It adds a new one. This gives us the ability to do some fun statistics. For example:

- Show builds with high/low success rate
- Show attempts needed for a build to pass
- Show build duration changes over time

We can use this to find flaky builds or prove builds got flakier or less flaky over time.

This repository contains:

- Scripts to download the commit status information from GitHub.
- Scripts to aggregate stats and write the to an InfluxDB.
- A Grafana dashboard for these stats.
- A Docker setup for a local InfluxDB and Grafana container.

![](docs/preview.png)

## Usage

### Start InfluxDB and Grafana

```
docker-compose up
```

### Download data and write to InfluxDB

On first run this will download the status information from GitHub and store in in a `data/` folder. Conescutive runs will then work based on the local data.

Setup local `.env` file with the following keys:

| Key                    | Value                                                                                                                                                                                                            |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GH_TOKEN`             | Personal access token for GitHub API. Needs permissions to read repo information.                                                                                                                                |
| `GH_OWNER`             | GitHub repo owner                                                                                                                                                                                                |
| `GH_REPO`              | GitHub repo name                                                                                                                                                                                                 |
| `GH_COMMITS_SINCE`     | Timestamp from which to download status information, e.g. `2019-08-01T00:00:00Z`                                                                                                                                 |
| `GH_COMMITS_UNTIL`     | Timestamp until which to download status information, e.g. `2019-10-10T00:00:00Z`                                                                                                                                |
| `BUILD_NAME_TRANSFORM` | Optional expression to transform build names (`status.context`). Format: `s/SEARCH/REPLACE/`. Spaces and slashes inside seach and replace have to be escaped. Can include multiple space separated instructions. |

Then install dependencies and run the script:

```
yarn
yarn compile
node build/index.js
```

### Open dashboard

<http://localhost:3000/d/Ff_zt3hWk>
