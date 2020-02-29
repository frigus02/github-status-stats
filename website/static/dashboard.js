const repository = document.querySelector('script[src="/static/dashboard.js"]')
  .dataset.repository;

const startDateInput = document.querySelector("#startdate");
startDateInput.valueAsNumber = Date.now() - 2592000000; // 30 days in milliseconds
const endDateInput = document.querySelector("#enddate");
endDateInput.valueAsNumber = Date.now();

const startOfDay = date => {
  const d = new Date(date);
  d.setHours(0);
  d.setMinutes(0);
  d.setSeconds(0);
  d.setMilliseconds(0);
  return d;
};

const endOfDay = date => {
  const d = new Date(date);
  d.setHours(23);
  d.setMinutes(59);
  d.setSeconds(59);
  d.setMilliseconds(999);
  return d;
};

const closestDurationUnit = ms => {
  const s = Math.round(ms / 1000);

  const m = Math.round(s / 60);
  if (m === 0 || Math.abs(m * 60 - s) > 15) {
    return `${s}s`;
  }

  const h = Math.round(m / 60);
  if (h === 0 || Math.abs(h * 60 - m) > 15) {
    return `${m}m`;
  }

  const d = Math.round(h / 24);
  if (d === 0 || Math.abs(d * 24 - h) > 6) {
    return `${h}h`;
  }

  const w = Math.round(d / 7);
  if (w === 0 || Math.abs(w * 7 - d) > 2) {
    return `${d}d`;
  }

  return `${w}w`;
};

const timeRange = () => {
  const startDate = startOfDay(startDateInput.valueAsDate);
  const endDate = endOfDay(endDateInput.valueAsDate);
  const range = endDate.getTime() - startDate.getTime();
  if (range <= 0) {
    throw new Error(
      `Invalid time range ${range}ms. Select an end date, with is equal to or after the start date.`
    );
  }

  return {
    filter: `time >= ${startDate.getTime()}ms AND time <= ${endDate.getTime()}ms`,
    // Roughly 720 entries
    groupByDetailed: `time(${closestDurationUnit(range / 720)})`,
    // Roughly 120 entries
    groupBySparse: `time(${closestDurationUnit(range / 120)})`
  };
};

const queryData = async rawQuery => {
  const time = timeRange();
  const query = rawQuery
    .replace("__time_filter__", time.filter)
    .replace("__time_group_sparse__", time.groupBySparse)
    .replace("__time_group_detailed__", time.groupByDetailed)
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

const onResize = cb => window.addEventListener("resize", throttle(cb, 100));

const onTimeRangeChange = cb => {
  let running = false;
  let again = false;
  const onChange = async () => {
    if (
      startDateInput.value &&
      endDateInput.value &&
      startDateInput.valueAsNumber <= endDateInput.valueAsNumber
    ) {
      if (running) {
        again = true;
      } else {
        again = false;
        running = true;
        await cb();
        running = false;
        if (again) {
          onChange();
        }
      }
    }
  };
  startDateInput.addEventListener("change", onChange);
  endDateInput.addEventListener("change", onChange);
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

  const statEl = document.createElement("div");
  element.appendChild(statEl);
  statEl.className = "single-stat";
  statEl.textContent = valueFormat(emptyData[1][0]);

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
  onResize(() => plot.setSize(getSize()));

  const loadData = async () => {
    const rawStat = await queryData(statQuery);
    const stat = prepareData(rawStat, valueTransform);
    statEl.textContent = valueFormat(stat[1][0]);

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
          value: (_self, rawValue) => valueFormat(rawValue),
          stroke: color(i)
        }))
      ],
      axes: [
        {},
        {
          values: (_self, ticks) => ticks.map(valueFormat)
        }
      ]
    };
    plot = new uPlot.Line(opts, data, element);
  };

  recreatePlot([], emptyData);
  onResize(() => plot.setSize(getSize()));

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
        value.textContent = valueFormat(data[i + 1][0]);
        tr.append(label, value);
        return tr;
      })
    );
  };
  loadData();
  onTimeRangeChange(loadData);
};

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
    GROUP BY __time_group_sparse__
  `,
  valueTransform: value => value * 100,
  valueFormat: value => `${value.toFixed(2)}%`,
  elementSelector: "#overall-success"
});

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
    GROUP BY __time_group_sparse__
  `,
  valueTransform: value => value / 1000 / 60,
  valueFormat: value => `${value.toFixed(2)} min`,
  elementSelector: "#overall-duration"
});

tablePanel({
  title: "Success rate by pipeline",
  query: `
    SELECT mean("successful") AS value
    FROM "build"
    WHERE __time_filter__
    GROUP BY "name"
  `,
  valueTransform: value => value * 100,
  valueFormat: value => `${value.toFixed(2)}%`,
  valueColumnName: "Success",
  labelTag: "name",
  labelColumnName: "Pipeline",
  elementSelector: "#success-by-pipeline"
});

tablePanel({
  title: "Duration by pipeline",
  query: `
    SELECT mean("duration_ms") AS value
    FROM "build"
    WHERE __time_filter__
    GROUP BY "name"
  `,
  valueTransform: value => value / 1000 / 60,
  valueFormat: value => `${value.toFixed(2)} min`,
  valueColumnName: "Duration",
  labelTag: "name",
  labelColumnName: "Pipeline",
  elementSelector: "#duration-by-pipeline"
});

graphPanel({
  title: "Duration",
  query: `
    SELECT mean("duration_ms") AS value
    FROM "build"
    WHERE __time_filter__
    GROUP BY __time_group_detailed__, "name"
  `,
  valueTransform: value => value / 1000 / 60,
  valueFormat: value => `${value.toFixed(2)} min`,
  labelTag: "name",
  elementSelector: "#duration"
});
