import fetch, { RequestInit } from "node-fetch";

const baseUrl = "http://localhost:8086";
const db = "db0";
const user = "user";
const password = "password";
const auth = Buffer.from(`${user}:${password}`).toString("base64");

const callInflux = async (path: string, options: RequestInit = {}) => {
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

export const dropMeasurement = (measurement: string) =>
  callInflux(`/query`, {
    method: "POST",
    headers: {
      "content-type": "application/x-www-form-urlencoded"
    },
    body: `q=DROP MEASUREMENT ${measurement}`
  });

export const write = (data: string) =>
  callInflux("/write", {
    method: "POST",
    body: data
  });

export const toInfluxTimestamp = (isoDate: string) =>
  new Date(isoDate).getTime() * 1000 * 1000;
