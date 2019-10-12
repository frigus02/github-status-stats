const fetch = require("node-fetch");

const baseUrl = "http://localhost:8086";
const db = "db0";
const user = "user";
const password = "password";
const auth = Buffer.from(`${user}:${password}`).toString("base64");

const callInflux = async (path, options = {}) => {
  const url = `${baseUrl}${path}?db=${db}`;
  const optionsWithAuth = {
    ...options,
    headers: {
      authorization: `Basic ${auth}`,
      ...options.headers
    }
  };
  console.log(`Calling ${url}`);
  const res = await fetch(url, optionsWithAuth);
  if (!res.ok) {
    throw new Error(`Call to InfluxDB ${path} returned ${res.status}`);
  }
};

const dropMeasurement = measurement =>
  callInflux(`/query`, {
    method: "POST",
    headers: {
      "content-type": "application/x-www-form-urlencoded"
    },
    body: `q=DROP MEASUREMENT ${measurement}`
  });

const write = data =>
  callInflux("/write", {
    method: "POST",
    body: data
  });

const toInfluxTimestamp = isoDate => new Date(isoDate).getTime() * 1000 * 1000;

module.exports = {
  dropMeasurement,
  write,
  toInfluxTimestamp
};
