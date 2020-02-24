// Success rate by pipeline
// SELECT mean("successful") FROM "build" WHERE "name" =~ /^build$/ AND time >= now() - 7d GROUP BY "name"
// | Pipeline | Successful |
// | -------- | ---------- |
// | build    | 50.00%     |

// Duration by pipeline
// SELECT mean("duration_ms") FROM "build" WHERE "name" =~ /^build$/ AND time >= now() - 7d GROUP BY "name"
// | Pipeline | Duration |
// | -------- | -------- |
// | build    | 2.66 min |

const repository = document.querySelector('script[src="/static/dashboard.js"]')
  .dataset.repository;

const query = async query => {
  const url = new URL("/api/query", location);
  url.searchParams.append("repository", repository);
  url.searchParams.append("query", query);
  const res = await fetch(url.toString());
  return res.json();
};

const overallSuccessRate = async () => {
  const raw = await query(`
    SELECT mean("successful")
    FROM "build"
    WHERE "name" =~ /^build$/ AND time >= now() - 30d
    GROUP BY time(6h)
  `);
  const time = raw.columns.indexOf("time");
  const mean = raw.columns.indexOf("mean");
  const data = [Array(raw.values.length), Array(raw.values.length)];
  for (let i = 0; i < raw.values.length; i++) {
    data[0][i] = Math.round(new Date(raw.values[i][time]).getTime() / 1000);
    data[1][i] = raw.values[i][mean];
  }

  const opts = {
    title: "Overall success rate",
    width: 370,
    height: 98,
    legend: { show: false },
    cursor: { show: false },
    series: [
      {},
      {
        spanGaps: true,
        stroke: "#1f5f95",
        fill: "rgba(31, 95, 149, .1)"
      }
    ],
    scales: { x: { time: false } },
    axes: [{ show: false }, { show: false }]
  };

  new uPlot.Line(opts, data, document.querySelector("#overall-success"));
};

const overallAverageDuration = async () => {
  const raw = await query(`
    SELECT mean("duration_ms")
    FROM "build"
    WHERE "name" =~ /^build$/ AND time >= now() - 30d
    GROUP BY time(6h)
  `);
  const time = raw.columns.indexOf("time");
  const mean = raw.columns.indexOf("mean");
  const data = [Array(raw.values.length), Array(raw.values.length)];
  for (let i = 0; i < raw.values.length; i++) {
    data[0][i] = Math.round(new Date(raw.values[i][time]).getTime() / 1000);
    data[1][i] = raw.values[i][mean];
  }

  const opts = {
    title: "Overall average duration",
    width: 370,
    height: 98,
    legend: { show: false },
    cursor: { show: false },
    series: [
      {},
      {
        spanGaps: true,
        stroke: "#1f5f95",
        fill: "rgba(31, 95, 149, .1)"
      }
    ],
    scales: { x: { time: false } },
    axes: [{ show: false }, { show: false }]
  };

  new uPlot.Line(opts, data, document.querySelector("#overall-duration"));
};

const duration = async () => {
  const raw = await query(`
    SELECT mean("duration_ms")
    FROM "build"
    WHERE "name" =~ /^build$/ AND time >= now() - 30d
    GROUP BY time(1h), "name"
  `);
  const time = raw.columns.indexOf("time");
  const mean = raw.columns.indexOf("mean");
  const data = [Array(raw.values.length), Array(raw.values.length)];
  for (let i = 0; i < raw.values.length; i++) {
    data[0][i] = Math.round(new Date(raw.values[i][time]).getTime() / 1000);
    data[1][i] = raw.values[i][mean];
  }

  const opts = {
    title: "Duration",
    width: 764,
    height: 362,
    series: [
      {},
      {
        label: "build", // TODO: get from tags from query result
        stroke: "#1f5f95" // TODO: get from color palette
      }
    ]
  };

  new uPlot.Line(opts, data, document.querySelector("#duration"));
};

overallSuccessRate();
overallAverageDuration();
duration();
