import { promises as fsPromises } from "fs";
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

const { writeFile } = fsPromises;

const callGitHub = async (pathOrUrl: string, options: RequestInit = {}) => {
  const url = pathOrUrl.startsWith("https://")
    ? pathOrUrl
    : `https://api.github.com${pathOrUrl}`;
  const optionsWithAuth = {
    ...options,
    headers: {
      authorization: `token ${env("GH_TOKEN")}`,
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

const getCommits = (): Promise<Commit[]> =>
  callGitHub(
    `/repos/${env("GH_OWNER")}/${env("GH_REPO")}/commits?since=${env(
      "GH_COMMITS_SINCE"
    )}&until=${env("GH_COMMITS_UNTIL")}`
  );

const getStatuses = (ref: string): Promise<CommitStatus[]> =>
  callGitHub(
    `/repos/${env("GH_OWNER")}/${env("GH_REPO")}/commits/${ref}/statuses`
  );

export const loadCommits = async () => {
  const commits = await getCommits();
  await writeFile(
    "data/commits.json",
    JSON.stringify(commits, null, 4),
    "utf8"
  );
  return commits;
};

export const loadStatuses = async (commit: Commit) => {
  const statuses = await getStatuses(commit.sha);
  await writeFile(
    `data/statuses-${commit.sha}.json`,
    JSON.stringify(statuses, null, 4),
    "utf8"
  );
  return statuses;
};
