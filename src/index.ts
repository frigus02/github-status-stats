import "dotenv/config";
import { DateTime } from "luxon";
import { optionalEnv } from "./env";
import { loadCommits, loadStatuses, CommitStatus } from "./github";
import {
  toInfluxTimestamp,
  write as writeToInfluxDB,
  Point,
  dropMeasurement
} from "./influxdb";
import { transformStatusContext } from "./transform";

interface Build {
  name: string;
  successful: boolean;
  canceled: boolean;
  duration_ms: number;
  created_at: string;
}

interface BuildAggregate {
  [name: string]: {
    attempts: number;
    first_attempt_successful: boolean;
  };
}

const isCanceled = ({ description }: Pick<CommitStatus, "description">) =>
  optionalEnv.BUILD_CANCELED_REGEX
    ? new RegExp(optionalEnv.BUILD_CANCELED_REGEX).test(description)
    : false;

const toBuilds = (statuses: CommitStatus[]): Build[] =>
  statuses
    .sort(
      (a, b) =>
        DateTime.fromISO(a.created_at).toMillis() -
        DateTime.fromISO(b.created_at).toMillis()
    )
    .reduce(
      (groups, currStatus) => {
        const group = groups.find(group =>
          group.every(
            status =>
              status.context === currStatus.context &&
              status.state === "pending"
          )
        );
        if (group) {
          group.push(currStatus);
        } else {
          groups.unshift([currStatus]);
        }

        return groups;
      },
      <CommitStatus[][]>[]
    )
    .reverse()
    .map(group => {
      const first = group[0];
      const last = group[group.length - 1];
      return {
        name: transformStatusContext(first.context),
        successful: last.state === "success",
        canceled: isCanceled(last),
        duration_ms:
          DateTime.fromISO(last.created_at).toMillis() -
          DateTime.fromISO(first.created_at).toMillis(),
        created_at: first.created_at
      };
    });

const accumulateBuilds = (sortedBuilds: Build[]): BuildAggregate =>
  sortedBuilds.reduce(
    (builds, build: Build) => {
      const existingBuild = builds[build.name];
      if (!existingBuild) {
        builds[build.name] = {
          attempts: 1,
          first_attempt_successful: build.successful
        };
      } else {
        existingBuild.attempts++;
      }

      return builds;
    },
    <BuildAggregate>{}
  );

const main = async () => {
  const commits = await loadCommits();
  const commitsCount = commits.length;
  const influxPoints: Point[] = [];
  for (const [i, commit] of commits.entries()) {
    console.log(`Commit ${i + 1}/${commitsCount}`);

    const statuses = await loadStatuses(commit);
    const builds = toBuilds(statuses);
    const accBuilds = accumulateBuilds(builds);

    influxPoints.push(
      ...builds.map(x => ({
        measurement: "build",
        tags: new Map([["name", x.name], ["commit", commit.sha]]),
        fields: new Map([
          ["successful", x.successful ? 1 : 0],
          ["canceled", x.canceled ? 1 : 0],
          ["duration_ms", x.duration_ms]
        ]),
        timestamp: toInfluxTimestamp(x.created_at)
      }))
    );

    influxPoints.push(
      ...Array.from(Object.entries(accBuilds)).map(x => ({
        measurement: "build_per_commit",
        tags: new Map([["name", x[0]], ["commit", commit.sha]]),
        fields: new Map([
          ["attempts", x[1].attempts],
          ["first_attempt_successful", x[1].first_attempt_successful ? 1 : 0]
        ]),
        timestamp: toInfluxTimestamp(commit.commit.committer.date)
      }))
    );
  }

  await dropMeasurement("build");
  await dropMeasurement("build_per_commit");
  await new Promise(resolve => setTimeout(resolve, 5000));
  await writeToInfluxDB(influxPoints);
};

main().catch(err => {
  console.error(err);
  process.exitCode = 1;
});
