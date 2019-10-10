const fetch = require("node-fetch");

const writeToInfluxDB = async data => {
  const url = "http://localhost:8086/write?db=db0";
  console.log(`Calling ${url}`);
  const res = await fetch(url, {
    method: "POST",
    headers: {
      authorization: `Basic ${Buffer.from("user:password").toString("base64")}`
    },
    body: data
  });

  if (!res.ok) {
    throw new Error(`Write to InfluxDB returned ${res.status}`);
  }
};

const toInfluxTimestamp = isoDate => new Date(isoDate).getTime() * 1000 * 1000;

module.exports = {
  writeToInfluxDB,
  toInfluxTimestamp
};
