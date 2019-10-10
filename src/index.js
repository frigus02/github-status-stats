require("dotenv").config();

const { readFile } = require("fs").promises;
const { loadCommits, loadStatuses } = require("./github");
const { toInfluxTimestamp, writeToInfluxDB } = require("./influxdb");

const accumulateBuilds = statuses =>
  statuses
    .filter(s => s.state !== "pending")
    .reduce((acc, status) => {
      const context = status.context;
      if (!acc[context]) {
        acc[context] = {
          attempts: 0,
          first_attempted_at: Number.MAX_VALUE
        };
      }

      acc[context].attempts++;
      acc[context].first_attempted_at = Math.min(
        acc[context].first_attempted_at,
        toInfluxTimestamp(status.created_at)
      );
      return acc;
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

    const data = Array.from(Object.entries(builds))
      .map(
        ([build, data]) =>
          `build,name=${build},commit=${commit.sha} attempts=${
            data.attempts
          } ${toInfluxTimestamp(commit.commit.committer.date)}`
      )
      .join("\n");
    await writeToInfluxDB(data);
  }
};

main().catch(err => {
  console.error(err);
  process.exitCode = 1;
});
