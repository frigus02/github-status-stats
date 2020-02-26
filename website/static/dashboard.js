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

const queryData = async query => {
  const url = new URL("/api/query", location);
  url.searchParams.append("repository", repository);
  url.searchParams.append("query", query);
  const res = await fetch(url.toString());
  return res.json();
};

const prepareData = (raw, yColumnName, valueTransform) => {
  const x = raw[0].columns.indexOf("time");
  const y = raw[0].columns.indexOf(yColumnName);
  const data = [];
  data.push(
    raw[0].values.map(row => Math.round(new Date(row[x]).getTime() / 1000))
  );
  for (const series of raw) {
    data.push(series.values.map(row => valueTransform(row[y])));
  }

  return data;
};

const statPanel = async ({
  title,
  statQuery,
  backgroundQuery,
  valueTransform,
  valueFormat,
  elementSelector
}) => {
  const rawStat = await queryData(statQuery);
  const stat = prepareData(rawStat, "mean", valueTransform)[1][0];

  const rawBackground = await queryData(backgroundQuery);
  const data = prepareData(rawBackground, "mean", valueTransform);
  const opts = {
    title,
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

  const element = document.querySelector(elementSelector);
  new uPlot.Line(opts, data, element);

  const statEl = document.createElement("div");
  element.appendChild(statEl);
  statEl.className = "single-stat";
  statEl.textContent = valueFormat.format(stat);
};

const graphPanel = async ({
  title,
  query,
  valueTransform,
  valueFormat,
  labelTag,
  elementSelector
}) => {
  const raw = await queryData(query);
  const data = prepareData(raw, "mean", valueTransform);
  const opts = {
    title,
    width: 764,
    height: 362,
    series: [
      {},
      ...raw.map(series => ({
        label: series.tags[labelTag],
        value: (_self, rawValue) => valueFormat.format(rawValue),
        stroke: "#1f5f95" // TODO: get from color palette
      }))
    ],
    axes: [
      {},
      {
        values: (_self, ticks) =>
          ticks.map(rawValue => valueFormat.format(rawValue))
      }
    ]
  };

  new uPlot.Line(opts, data, document.querySelector(elementSelector));
};

const overallSuccessRate = () =>
  statPanel({
    title: "Overall success rate",
    statQuery: `
      SELECT mean("successful")
      FROM "build"
      WHERE "name" =~ /^build$/ AND time >= now() - 30d
    `,
    backgroundQuery: `
      SELECT mean("successful")
      FROM "build"
      WHERE "name" =~ /^build$/ AND time >= now() - 30d
      GROUP BY time(6h)
    `,
    valueTransform: value => value,
    valueFormat: new Intl.NumberFormat(undefined, {
      style: "unit",
      unit: "percent",
      maximumFractionDigits: 2
    }),
    elementSelector: "#overall-success"
  });

const overallAverageDuration = () =>
  statPanel({
    title: "Overall average duration",
    statQuery: `
      SELECT mean("duration_ms")
      FROM "build"
      WHERE "name" =~ /^build$/ AND time >= now() - 30d
    `,
    backgroundQuery: `
      SELECT mean("duration_ms")
      FROM "build"
      WHERE "name" =~ /^build$/ AND time >= now() - 30d
      GROUP BY time(6h)
    `,
    valueTransform: value => value / 1000 / 60,
    valueFormat: new Intl.NumberFormat(undefined, {
      style: "unit",
      unit: "minute",
      maximumFractionDigits: 2
    }),
    elementSelector: "#overall-duration"
  });

const duration = () =>
  graphPanel({
    title: "Duration",
    query: `
      SELECT mean("duration_ms")
      FROM "build"
      WHERE "name" =~ /^build$/ AND time >= now() - 30d
      GROUP BY time(1h), "name"
    `,
    valueTransform: value => value / 1000 / 60,
    valueFormat: new Intl.NumberFormat(undefined, {
      style: "unit",
      unit: "minute",
      maximumFractionDigits: 2
    }),
    labelTag: "name",
    elementSelector: "#duration"
  });

overallSuccessRate();
overallAverageDuration();
duration();
