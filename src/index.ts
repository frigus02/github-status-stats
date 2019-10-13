import "dotenv/config";
import { loadCommits, loadStatuses, CommitStatus } from "./github";
import { toInfluxTimestamp, write as writeToInfluxDB } from "./influxdb";
import { transformBuildName } from "./transform";

interface Build {
  name: string;
  successful: boolean;
  date: number;
}

interface BuildAggregate {
  [name: string]: {
    attempts: number;
    first_attempt_successful: boolean;
    first_attempted_at: number;
  };
}

const accumulateBuilds = (statuses: CommitStatus[]): BuildAggregate =>
  statuses
    .filter(status => status.state !== "pending")
    .map(status => ({
      name: transformBuildName(status.context),
      successful: status.state === "success",
      date: toInfluxTimestamp(status.created_at)
    }))
    .reduce(
      (builds, build: Build) => {
        const existingBuild = builds[build.name];
        if (!existingBuild) {
          builds[build.name] = {
            attempts: 1,
            first_attempt_successful: build.successful,
            first_attempted_at: build.date
          };
        } else {
          existingBuild.attempts++;
          if (build.date < existingBuild.first_attempted_at) {
            existingBuild.first_attempt_successful = build.successful;
            existingBuild.first_attempted_at = build.date;
          }
        }
        return builds;
      },
      <BuildAggregate>{}
    );

const main = async () => {
  const commits = await loadCommits();
  const commitsCount = commits.length;
  const influxRows: string[] = [];
  for (const [i, commit] of commits.entries()) {
    console.log(`Commit ${i + 1}/${commitsCount}`);

    const statuses = await loadStatuses(commit);
    const builds = accumulateBuilds(statuses);

    influxRows.push(
      ...Array.from(Object.entries(builds)).map(
        x =>
          `build,name=${x[0]},commit=${commit.sha} attempts=${
            x[1].attempts
          },first_attempt_successful=${
            x[1].first_attempt_successful ? 1 : 0
          } ${toInfluxTimestamp(commit.commit.committer.date)}`
      )
    );
  }

  // await dropMeasurement("build");
  await writeToInfluxDB(influxRows.join("\n"));
};

main().catch(err => {
  console.error(err);
  process.exitCode = 1;
});
