require("dotenv").config();

const { readFile } = require("fs").promises;
const { loadCommits, loadStatuses } = require("./github");
const {
  dropMeasurement,
  toInfluxTimestamp,
  write: writeToInfluxDB
} = require("./influxdb");
const { transformBuildName } = require("./transform");

const accumulateBuilds = statuses =>
  statuses
    .filter(status => status.state !== "pending")
    .reduce((builds, status) => {
      const name = transformBuildName(status.context);
      const createdAt = toInfluxTimestamp(status.created_at);
      const successful = status.state === "success";
      if (!builds[name]) {
        builds[name] = {
          attempts: 1,
          first_attempt_successful: successful,
          first_attempted_at: createdAt
        };
      } else {
        builds[name].attempts++;
        if (createdAt < builds[name].first_attempted_at) {
          builds[name].first_attempt_successful = successful;
          builds[name].first_attempted_at = createdAt;
        }
      }
      return builds;
    }, {});

const main = async () => {
  let commits;
  try {
    commits = JSON.parse(await readFile("data/commits.json", "utf8"));
  } catch (err) {
    if (err.code === "ENOENT") {
      commits = await loadCommits();
    } else {
      throw err;
    }
  }

  const commitsCount = commits.length;
  const influxRows = [];
  for (const [i, commit] of commits.entries()) {
    console.log(`Commit ${i + 1}/${commitsCount}`);

    let statuses;
    try {
      statuses = JSON.parse(
        await readFile(`data/statuses-${commit.sha}.json`, "utf8")
      );
    } catch (err) {
      if (err.code === "ENOENT") {
        statuses = await loadStatuses(commit);
      } else {
        throw err;
      }
    }

    const builds = accumulateBuilds(statuses);

    influxRows.push(
      ...Array.from(Object.entries(builds)).map(
        ([build, data]) =>
          `build,name=${build},commit=${commit.sha} attempts=${
            data.attempts
          },first_attempt_successful=${
            data.first_attempt_successful ? 1 : 0
          } ${toInfluxTimestamp(commit.commit.committer.date)}`
      )
    );
  }

  await dropMeasurement("build");
  await writeToInfluxDB(influxRows.join("\n"));
};

main().catch(err => {
  console.error(err);
  process.exitCode = 1;
});
