const repository = document.querySelector('script[src="/static/dashboard.js"]')
  .dataset.repository;

const endDateInput = document.querySelector("#enddate");
endDateInput.valueAsDate = new Date();

const timeRange = () => {
  const endDate = endDateInput.valueAsDate;
  endDate.setHours(23);
  endDate.setMinutes(59);
  endDate.setSeconds(59);
  endDate.setMilliseconds(999);
  const range = 2592000000; // 30 days in milliseconds
  const startDate = endDate.getTime() - range;

  return `time > ${startDate}ms AND time <= ${endDate.getTime()}ms`;
};

const queryData = async rawQuery => {
  const query = rawQuery
    .replace("__time_filter__", timeRange())
    .replace(/[\n\s]+/g, " ")
    .trim();

  const url = new URL("/api/query", location);
  url.searchParams.append("repository", repository);
  url.searchParams.append("query", query);
  const res = await fetch(url.toString());
  if (!res.ok) {
    throw new Error(
      `Query failed eith ${res.status} ${res.statusText} (query=${query})`
    );
  }

  return res.json();
};

const emptyData = [[0], [Number.NaN]];

const prepareData = (raw, valueTransform) => {
  if (raw.length === 0) {
    return emptyData;
  }

  const x = raw[0].columns.indexOf("time");
  const y = raw[0].columns.indexOf("value");
  const data = [];
  data.push(
    raw[0].values.map(row => Math.round(new Date(row[x]).getTime() / 1000))
  );
  for (const series of raw) {
    data.push(series.values.map(row => valueTransform(row[y])));
  }

  return data;
};

const onResize = cb => window.addEventListener("resize", cb);

const onTimeRangeChange = cb =>
  endDateInput.addEventListener("change", () => {
    if (endDateInput.value) {
      cb();
    }
  });

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

  const statEl = document.createElement("div");
  element.appendChild(statEl);
  statEl.className = "single-stat";
  statEl.textContent = valueFormat.format(emptyData[1][0]);

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

  const plot = new uPlot.Line(opts, emptyData, element);
  onResize(throttle(() => plot.setSize(getSize()), 100));

  const loadData = async () => {
    const rawStat = await queryData(statQuery);
    const stat = prepareData(rawStat, valueTransform);
    statEl.textContent = valueFormat.format(stat[1][0]);

    const rawBackground = await queryData(backgroundQuery);
    const data = prepareData(rawBackground, valueTransform);
    plot.setData(data);
  };
  loadData();
  onTimeRangeChange(loadData);
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

  const getSize = () => getUPlotSize(element, 375);

  let plot;
  const recreatePlot = (raw, data) => {
    if (plot) {
      plot.destroy();
    }

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
    plot = new uPlot.Line(opts, data, element);
  };

  recreatePlot([], emptyData);
  onResize(throttle(() => plot.setSize(getSize()), 100));

  const loadData = async () => {
    const raw = await queryData(query);
    const data = prepareData(raw, valueTransform);
    recreatePlot(raw, data);
  };
  loadData();
  onTimeRangeChange(loadData);
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

  const table = document.createElement("table");
  table.className = "table-stat";
  table.append(caption, thead, tbody);

  const element = document.querySelector(elementSelector);
  element.append(table);

  const loadData = async () => {
    const raw = await queryData(query);
    const data = prepareData(raw, valueTransform);

    while (tbody.firstChild) {
      tbody.removeChild(tbody.firstChild);
    }

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
  };
  loadData();
  onTimeRangeChange(loadData);
};

const overallSuccessRate = () =>
  statPanel({
    title: "Overall success rate",
    statQuery: `
      SELECT mean("successful") AS value
      FROM "build"
      WHERE __time_filter__
    `,
    backgroundQuery: `
      SELECT mean("successful") AS value
      FROM "build"
      WHERE __time_filter__
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
      SELECT mean("duration_ms") AS value
      FROM "build"
      WHERE __time_filter__
    `,
    backgroundQuery: `
      SELECT mean("duration_ms") AS value
      FROM "build"
      WHERE __time_filter__
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
      SELECT mean("successful") AS value
      FROM "build"
      WHERE __time_filter__
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
      SELECT mean("duration_ms") AS value
      FROM "build"
      WHERE __time_filter__
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
      SELECT mean("duration_ms") AS value
      FROM "build"
      WHERE __time_filter__
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
