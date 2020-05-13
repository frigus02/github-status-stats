const fs = require("fs");

const rawData = fs.readFileSync("exports-build.json", "utf8");
const data = JSON.parse(rawData);

// import
//console.log("timestamp");
//for (const row of data[0].values) {
//	console.log(new Date(row[0]).getTime());
//}

// hook
//console.log("timestamp,type,commit");
//for (const row of data[0].values) {
//  console.log(
//    `${new Date(row[0]).getTime()},${row[2] === "check_run" ? 2 : 1},${row[1]}`
//  );
//}

// commit
//console.log("commit,build_name,build_source,builds,builds_successful,builds_failed,timestamp");
//for (const row of data[0].values) {
//	console.log(`${row[6]},${row[1]},${row[2] === "check_run" ? 2 : 1},${row[3]},${row[5]},${row[4]},${new Date(row[0]).getTime()}`);
//}

// build
console.log("commit,name,source,timestamp,successful,failed,duration_ms");
for (const row of data[0].values) {
	console.log(`${row[1]},${row[4]},${row[5] === "check_run" ? 2 : 1},${new Date(row[0]).getTime()},${row[6]},${row[3] === null ? 0 : row[3]},${row[2]}`);
}
