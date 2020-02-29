const repository = document.querySelector('script[src="/static/dashboard.js"]')
  .dataset.repository;
const filters = [
  // '"name" =~ /^build$/',
  "time >= now() - 30d"
];

const queryData = async query => {
  const url = new URL("/api/query", location);
  url.searchParams.append("repository", repository);
  url.searchParams.append("query", query);
  const res = await fetch(url.toString());
  if (!res.ok) {
    throw new Error(
      `Query failed eith ${res.status} ${res.statusText} (query=${query
        .replace(/[\n\s]+/g, " ")
        .trim()})`
    );
  }

  return res.json();
};

const prepareData = (raw, yColumnName, valueTransform) => {
  if (raw.length === 0) {
    return [[0], [Number.NaN]];
  }

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

const color = (index, alpha = 1) =>
  `hsla(${index * 222.5}, 75%, 50%, ${alpha})`;

const throttle = (cb, limit) => {
  let wait = false;
  return () => {
    if (!wait) {
      wait = true;
      setTimeout(() => {
        requestAnimationFrame(cb);
        wait = false;
      }, limit);
    }
  };
};

const getUPlotSize = (element, height) => {
  const style = getComputedStyle(element);
  return {
    width:
      element.clientWidth -
      parseInt(style.paddingLeft, 10) -
      parseInt(style.paddingRight, 10),
    height
  };
};

const statPanel = async ({
  title,
  statQuery,
  backgroundQuery,
  valueTransform,
  valueFormat,
  elementSelector
}) => {
  const element = document.querySelector(elementSelector);

  const rawStat = await queryData(statQuery);
  const stat = prepareData(rawStat, "mean", valueTransform)[1][0];
  const statEl = document.createElement("div");
  element.appendChild(statEl);
  statEl.className = "single-stat";
  statEl.textContent = valueFormat.format(stat);

  const rawBackground = await queryData(backgroundQuery);
  const data = prepareData(rawBackground, "mean", valueTransform);
  const getSize = () => getUPlotSize(element, 100);
  const opts = {
    title,
    ...getSize(),
    legend: { show: false },
    cursor: { show: false },
    series: [
      {},
      {
        spanGaps: true,
        stroke: color(0),
        fill: color(0, 0.1)
      }
    ],
    scales: { x: { time: false } },
    axes: [{ show: false }, { show: false }]
  };

  const plot = new uPlot.Line(opts, data, element);
  window.addEventListener(
    "resize",
    throttle(() => plot.setSize(getSize()), 100)
  );
};

const graphPanel = async ({
  title,
  query,
  valueTransform,
  valueFormat,
  labelTag,
  elementSelector
}) => {
  const element = document.querySelector(elementSelector);
  const raw = await queryData(query);
  const data = prepareData(raw, "mean", valueTransform);
  const getSize = () => getUPlotSize(element, 375);
  const opts = {
    title,
    ...getSize(),
    series: [
      {},
      ...raw.map((series, i) => ({
        label: series.tags[labelTag],
        value: (_self, rawValue) => valueFormat.format(rawValue),
        stroke: color(i)
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

  const plot = new uPlot.Line(opts, data, element);
  window.addEventListener(
    "resize",
    throttle(() => plot.setSize(getSize()), 100)
  );
};

const tablePanel = async ({
  title,
  query,
  valueTransform,
  valueFormat,
  valueColumnName,
  labelTag,
  labelColumnName,
  elementSelector
}) => {
  const raw = await queryData(query);
  const data = prepareData(raw, "mean", valueTransform);

  const caption = document.createElement("caption");
  caption.textContent = title;

  const trHead = document.createElement("tr");
  const thLabel = document.createElement("th");
  thLabel.scope = "col";
  thLabel.textContent = labelColumnName;
  const thValue = document.createElement("th");
  thValue.scope = "col";
  thValue.textContent = valueColumnName;
  trHead.append(thLabel, thValue);

  const thead = document.createElement("thead");
  thead.append(trHead);

  const tbody = document.createElement("tbody");
  tbody.append(
    ...raw.map((series, i) => {
      const tr = document.createElement("tr");
      const label = document.createElement("th");
      label.scope = "row";
      label.textContent = series.tags[labelTag];
      const value = document.createElement("td");
      value.textContent = valueFormat.format(data[i + 1][0]);
      tr.append(label, value);
      return tr;
    })
  );

  const table = document.createElement("table");
  table.className = "table-stat";
  table.append(caption, thead, tbody);

  const element = document.querySelector(elementSelector);
  element.append(table);
};

const overallSuccessRate = () =>
  statPanel({
    title: "Overall success rate",
    statQuery: `
      SELECT mean("successful")
      FROM "build"
      WHERE ${filters.join(" AND ")}
    `,
    backgroundQuery: `
      SELECT mean("successful")
      FROM "build"
      WHERE ${filters.join(" AND ")}
      GROUP BY time(6h)
    `,
    valueTransform: value => value * 100,
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
      WHERE ${filters.join(" AND ")}
    `,
    backgroundQuery: `
      SELECT mean("duration_ms")
      FROM "build"
      WHERE ${filters.join(" AND ")}
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

const successByPipeline = () =>
  tablePanel({
    title: "Success rate by pipeline",
    query: `
      SELECT mean("successful")
      FROM "build"
      WHERE ${filters.join(" AND ")}
      GROUP BY "name"
    `,
    valueTransform: value => value * 100,
    valueFormat: new Intl.NumberFormat(undefined, {
      style: "unit",
      unit: "percent",
      maximumFractionDigits: 2
    }),
    valueColumnName: "Success",
    labelTag: "name",
    labelColumnName: "Pipeline",
    elementSelector: "#success-by-pipeline"
  });

const durationByPipeline = () =>
  tablePanel({
    title: "Duration by pipeline",
    query: `
      SELECT mean("duration_ms")
      FROM "build"
      WHERE ${filters.join(" AND ")}
      GROUP BY "name"
    `,
    valueTransform: value => value / 1000 / 60,
    valueFormat: new Intl.NumberFormat(undefined, {
      style: "unit",
      unit: "minute",
      maximumFractionDigits: 2
    }),
    valueColumnName: "Duration",
    labelTag: "name",
    labelColumnName: "Pipeline",
    elementSelector: "#duration-by-pipeline"
  });

const duration = () =>
  graphPanel({
    title: "Duration",
    query: `
      SELECT mean("duration_ms")
      FROM "build"
      WHERE ${filters.join(" AND ")}
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
successByPipeline();
durationByPipeline();
duration();
