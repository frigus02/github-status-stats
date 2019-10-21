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

The [`docker-compose.yml`](docker-compose.yml) starts an InfluxDB and Grafana with a preconfigured datasource and dashboard. After startup the dashboard should be available under <http://localhost:3000/d/Ff_zt3hWk>.

```
docker-compose up
```

### Download data and write to InfluxDB

Create a [configuration](#configuration). Then install dependencies and run the script.

```
yarn
yarn compile
node build/index.js
```

On first run this will download the status information from GitHub and store it in a `data/` folder. Conescutive runs will then work based on the local data.

## Configuration

Configuration works through environment variables. Create a `.env` file with the following keys. See [`.env.example`](.env.example) for an example file.

| Key                    | Value                                                                                                                                                                                                                                                                                                                                                                                                     | Required         |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| `GH_TOKEN`             | Personal access token for GitHub API. Needs permissions to read repo.                                                                                                                                                                                                                                                                                                                                     | Yes <sup>1</sup> |
| `GH_OWNER`             | GitHub repo owner, e.g. `frigus02`                                                                                                                                                                                                                                                                                                                                                                        | Yes <sup>1</sup> |
| `GH_REPO`              | GitHub repo name, e.g. `github-status-stats`                                                                                                                                                                                                                                                                                                                                                              | Yes <sup>1</sup> |
| `GH_COMMITS_SINCE`     | The tool will download commits and their statuses bewteen this and `GH_COMMITS_UNTIL`, e.g. `2019-08-01T00:00:00Z`                                                                                                                                                                                                                                                                                        | Yes              |
| `GH_COMMITS_UNTIL`     | The tool will download commits and their statuses between `GH_COMMITS_SINCE` and this, e.g. `2019-10-10T00:00:00Z`                                                                                                                                                                                                                                                                                        | Yes              |
| `BUILD_NAME_TRANSFORM` | Expression to transform build names (the `context` field from the [GitHub statuses API](https://developer.github.com/v3/repos/statuses/)). Can be used to remove common prefixes, normalize names when they changed over time and more. Syntax is similar to sed: `s/SEARCH/REPLACE/`. Spaces and slashes inside seach and replace have to be escaped. Can include multiple space separated instructions. | No               |

1. Commits and their statuses are cached locally in a `data/` folder. GitHub repo information and access token are only required if the commits inside the specified range don't already exist locally.
