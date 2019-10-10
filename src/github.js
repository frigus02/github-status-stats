const { writeFile } = require("fs").promises;
const fetch = require("node-fetch");
const parseLinkHeader = require("parse-link-header");
const env = require("./env");

const callGitHub = async (pathOrUrl, options = {}) => {
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

const getCommits = () =>
  callGitHub(
    `/repos/${env("GH_OWNER")}/${env("GH_REPO")}/commits?since=${env(
      "GH_COMMITS_SINCE"
    )}&until=${env("GH_COMMITS_UNTIL")}`
  );

const getStatuses = ref =>
  callGitHub(
    `/repos/${env("GH_OWNER")}/${env("GH_REPO")}/commits/${ref}/statuses`
  );

const loadCommits = async () => {
  const commits = await getCommits();
  await writeFile(
    "data/commits.json",
    JSON.stringify(commits, null, 4),
    "utf8"
  );
  return commits;
};

const loadStatuses = async commit => {
  const statuses = await getStatuses(commit.sha);
  await writeFile(
    `data/statuses-${commit.sha}.json`,
    JSON.stringify(statuses, null, 4),
    "utf8"
  );
  return statuses;
};

module.exports = {
  loadCommits,
  loadStatuses
};
