const fs = require("fs");
const https = require("https");

const fetch = (url, options) =>
  new Promise((resolve, reject) => {
    const req = https.request(url, options, (res) => {
      if (res.statusCode !== 200) {
        reject(new Error("HTTP status " + res.statusCode));
        return;
      }

      res.setEncoding("utf8");
      let body = "";
      res.on("data", (chunk) => {
        body += chunk;
      });
      res.on("end", () => {
        resolve(JSON.parse(body));
      });
    });
    req.on("error", (err) => {
      reject(err);
    });
    req.end();
  });

const baseUrl = "https://github-status-stats.kuehle.me/api/query";
const repositories = [214288339, 161389555];
const token = "xxx";

const main = async () => {
  for (const repository of repositories) {
    // Imports
    const imports = await fetch(
      `${baseUrl}?repository=${repository}&query=SELECT+*+FROM+%22import%22`,
      { headers: { Cookie: `token=${token}` } }
    );
    let importsCsv = "timestamp\n";
    for (const row of imports[0].values) {
      importsCsv += `${new Date(row[0]).getTime()}\n`;
    }
    fs.writeFileSync(`export-${repository}-imports.csv`, importsCsv, "utf8");

    // Hook
    const hooks = await fetch(
      `${baseUrl}?repository=${repository}&query=SELECT+*+FROM+%22hook%22`,
      { headers: { Cookie: `token=${token}` } }
    );
    let hooksCsv = "timestamp,type,commit\n";
    if (hooks.length > 0) {
      for (const row of hooks[0].values) {
        hooksCsv += `${new Date(row[0]).getTime()},${
          row[2] === "check_run" ? 2 : 1
        },${row[1]}\n`;
      }
    }
    fs.writeFileSync(`export-${repository}-hooks.csv`, hooksCsv, "utf8");

    // Commit
    const commits = await fetch(
      `${baseUrl}?repository=${repository}&query=SELECT+*+FROM+%22commit%22`,
      { headers: { Cookie: `token=${token}` } }
    );
    let commitsCsv =
      "commit,build_name,build_source,builds,builds_successful,builds_failed,timestamp\n";
    if (commits.length > 0) {
      for (const row of commits[0].values) {
        commitsCsv += `${row[6]},${row[1]},${row[2] === "check_run" ? 2 : 1},${
          row[3]
        },${row[5]},${row[4]},${new Date(row[0]).getTime()}\n`;
      }
    }
    fs.writeFileSync(`export-${repository}-commits.csv`, commitsCsv, "utf8");

    // Builds
    const builds = await fetch(
      `${baseUrl}?repository=${repository}&query=SELECT+*+FROM+%22build%22`,
      { headers: { Cookie: `token=${token}` } }
    );
    let buildsCsv =
      "commit,name,source,timestamp,successful,failed,duration_ms\n";
    const bCommit = builds[0].columns.indexOf("commit");
    const bName = builds[0].columns.indexOf("name");
    const bSource = builds[0].columns.indexOf("source");
    const bTime = builds[0].columns.indexOf("time");
    const bSuccessful = builds[0].columns.indexOf("successful");
    const bFailed = builds[0].columns.indexOf("failed");
    const bDuration = builds[0].columns.indexOf("duration_ms");
    for (const row of builds[0].values) {
      buildsCsv += `${row[bCommit]},${row[bName]},${
        row[bSource] === "check_run" ? 2 : 1
      },${new Date(row[bTime]).getTime()},${row[bSuccessful]},${
        row[bFailed] === null ? 0 : row[bFailed]
      },${row[bDuration]}\n`;
    }
    fs.writeFileSync(`export-${repository}-builds.csv`, buildsCsv, "utf8");
  }
};

main().catch(console.error);
