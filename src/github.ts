import { promises as fsPromises } from "fs";
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

export const loadCommits = async () => {
  let since = DateTime.fromISO(env.GH_COMMITS_SINCE, {
    zone: "utc"
  }).startOf("day");
  const until = DateTime.fromISO(env.GH_COMMITS_UNTIL, {
    zone: "utc"
  }).endOf("day");

  const commits = [];
  while (since < until) {
    const path = `data/commits/${since.toISO().replace(/[-:.]/g, "")}.json`;
    let commitsThisDay: Commit[];
    try {
      commitsThisDay = JSON.parse(await readFile(path, "utf8"));
    } catch (err) {
      if (err.code !== "ENOENT") throw err;

      commitsThisDay = await getCommits(
        since.toISO(),
        since.plus({ day: 1 }).toISO()
      );
      await mkdir("data/commits/", { recursive: true });
      await writeFile(path, JSON.stringify(commitsThisDay, null, 4), "utf8");
    }

    commits.push(...commitsThisDay);
    since = since.plus({ day: 1 });
  }

  return commits;
};

export const loadStatuses = async (commit: Commit) => {
  const path = `data/statuses/${commit.sha}.json`;
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (err) {
    if (err.code !== "ENOENT") throw err;

    const statuses = await getStatuses(commit.sha);
    await mkdir("data/statuses/", { recursive: true });
    await writeFile(path, JSON.stringify(statuses, null, 4), "utf8");
    return statuses;
  }
};
