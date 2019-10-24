import { promises as fsPromises } from "fs";
import { dirname } from "path";
import { DateTime } from "luxon";
import fetch, { RequestInit } from "node-fetch";
import * as parseLinkHeader from "parse-link-header";
import { env } from "./env";

export interface CommitPerson {
  name: string;
  email: string;
  date: string;
}

export interface Commit {
  sha: string;
  commit: {
    author: CommitPerson;
    committer: CommitPerson;
    message: string;
  };
}

export interface CommitStatus {
  state: "pending" | "success" | "failure";
  description: string;
  context: string;
  created_at: string;
  updated_at: string;
}

const { mkdir, readFile, writeFile } = fsPromises;

const callGitHub = async (pathOrUrl: string, options: RequestInit = {}) => {
  const url = pathOrUrl.startsWith("https://")
    ? pathOrUrl
    : `https://api.github.com${pathOrUrl}`;
  const optionsWithAuth = {
    ...options,
    headers: {
      authorization: `token ${env.GH_TOKEN}`,
      ...options.headers
    }
  };
  console.log(`Calling ${url}`);
  const res = await fetch(url, optionsWithAuth);
  if (!res.ok) {
    throw new Error(`Call to GitHub ${pathOrUrl} returned ${res.status}`);
  }

  const result = await res.json();
  const linkHeader = res.headers.get("link");
  const links = linkHeader && parseLinkHeader(linkHeader);
  if (Array.isArray(result) && links && links.next) {
    result.push(...(await callGitHub(links.next.url, options)));
  }

  return result;
};

const getCommits = (since: string, until: string): Promise<Commit[]> =>
  callGitHub(
    `/repos/${env.GH_OWNER}/${env.GH_REPO}/commits?since=${since}&until=${until}`
  );

const getStatuses = (ref: string): Promise<CommitStatus[]> =>
  callGitHub(`/repos/${env.GH_OWNER}/${env.GH_REPO}/commits/${ref}/statuses`);

const daysBetween = function*(since: string, until: string) {
  let sinceDateTime = DateTime.fromISO(since, {
    zone: "utc"
  }).startOf("day");
  const untilDateTime = DateTime.fromISO(until, {
    zone: "utc"
  }).endOf("day");
  while (sinceDateTime < untilDateTime) {
    yield sinceDateTime;
    sinceDateTime = sinceDateTime.plus({ day: 1 });
  }
};

const dataPath = (subPath: string) =>
  `data/${env.GH_OWNER}/${env.GH_REPO}/${subPath}`;

const readOrFetchAndWrite = async <T>(
  path: string,
  fetch: () => Promise<T>
): Promise<T> => {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (err) {
    if (err.code !== "ENOENT") throw err;

    const data = await fetch();
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, JSON.stringify(data, null, 4), "utf8");
    return data;
  }
};

const loadCommitsForDay = async (day: DateTime): Promise<Commit[]> =>
  readOrFetchAndWrite(
    dataPath(`commits/${day.toISO().replace(/[-:.]/g, "")}.json`),
    () => getCommits(day.toISO(), day.endOf("day").toISO())
  );

export const loadCommits = async () =>
  (await Promise.all(
    Array.from(daysBetween(env.GH_COMMITS_SINCE, env.GH_COMMITS_UNTIL)).map(
      loadCommitsForDay
    )
  )).flat();

export const loadStatuses = async (commit: Commit): Promise<CommitStatus[]> =>
  readOrFetchAndWrite(dataPath(`statuses/${commit.sha}.json`), () =>
    getStatuses(commit.sha)
  );
