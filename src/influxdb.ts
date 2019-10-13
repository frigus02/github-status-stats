import fetch, { RequestInit } from "node-fetch";

const baseUrl = "http://localhost:8086";
const db = "db0";
const user = "user";
const password = "password";
const auth = Buffer.from(`${user}:${password}`).toString("base64");

// Unix nanosecond timestamp.
export interface Timestamp extends Number {}

// https://docs.influxdata.com/influxdb/v1.7/write_protocols/line_protocol_reference/
export interface Point {
  measurement: string;
  tags: Map<string, string>;
  fields: Map<string, number | string | boolean>;
  timestamp: Timestamp;
}

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

const pointToLine = (point: Point) => {
  const measurementAndTags = [
    point.measurement,
    ...Array.from(point.tags.entries()).map(tag => `${tag[0]}=${tag[1]}`)
  ].join(",");

  const fields = Array.from(point.fields.entries())
    .map(field => `${field[0]}=${field[1]}`)
    .join(",");

  return [measurementAndTags, fields, point.timestamp].join(" ");
};

export const write = (points: Point[]) =>
  callInflux("/write", {
    method: "POST",
    body: points.map(pointToLine).join("\n")
  });

export const toInfluxTimestamp = (isoDate: string): Timestamp =>
  new Date(isoDate).getTime() * 1000 * 1000;
