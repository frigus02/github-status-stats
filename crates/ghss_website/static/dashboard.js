const repository = document.querySelector("#dashboard").dataset.repository;

const startDateInput = document.querySelector("#startdate");
startDateInput.valueAsNumber = Date.now() - 2592000000; // 30 days in milliseconds
const endDateInput = document.querySelector("#enddate");
endDateInput.valueAsNumber = Date.now();

const startOfDay = (date) => {
  const d = new Date(date);
  d.setHours(0);
  d.setMinutes(0);
  d.setSeconds(0);
  d.setMilliseconds(0);
  return d.getTime();
};

const endOfDay = (date) => {
  const d = new Date(date);
  d.setHours(23);
  d.setMinutes(59);
  d.setSeconds(59);
  d.setMilliseconds(999);
  return d.getTime();
};

const closestDurationUnit = (ms) => {
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
  const start = startOfDay(startDateInput.valueAsNumber);
  const end = endOfDay(endDateInput.valueAsNumber);
  return { start, end };
};

const timeRangeFilter = () => {
  const { start, end } = timeRange();
  const range = end - start;
  if (range <= 0) {
    throw new Error(
      `Invalid time range ${range}ms. Select an end date, with is equal to or after the start date.`
    );
  }

  return {
    filter: `time >= ${start}ms AND time <= ${end}ms`,
    // Roughly 720 entries
    groupByDetailed: `time(${closestDurationUnit(range / 720)})`,
    // Roughly 120 entries
    groupBySparse: `time(${closestDurationUnit(range / 120)})`,
  };
};

const queryData = async (rawQuery) => {
  const time = timeRangeFilter();
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

const emptyData = () => {
  const { start, end } = timeRange();
  return [
    [start / 1000, end / 1000],
    [null, null],
  ];
};

const prepareData = (raw, valueTransform) => {
  if (raw.length === 0) {
    return emptyData();
  }

  const x = raw[0].columns.indexOf("time");
  const y = raw[0].columns.indexOf("value");
  const data = [];
  data.push(
    raw[0].values.map((row) => Math.round(new Date(row[x]).getTime() / 1000))
  );
  for (const series of raw) {
    data.push(series.values.map((row) => valueTransform(row[y])));
  }

  return data;
};

const formatNumber = (n) =>
  n == null ? "" : n.toFixed(2).replace(/(\.0)?0$/, "");

const onResize = (cb) => window.addEventListener("resize", throttle(cb, 100));

const onTimeRangeChange = (cb) => {
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
  `hsla(${index * 222.5 + 348}, 100%, 51.4%, ${alpha})`;

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
    height,
  };
};

const createElement = (name, props = {}, children = []) => {
  const element = document.createElement(name);
  for (const [key, value] of Object.entries(props)) {
    if (key.includes("-")) {
      element.setAttribute(key, value);
    } else {
      element[key] = value;
    }
  }

  element.append(...children);
  return element;
};

const accessibilityPlugin = ({ ariaLabelledBy }) => {
  const thead = createElement("thead");
  const tbody = createElement("tbody");
  const table = createElement("table", { "aria-labelledby": ariaLabelledBy }, [
    thead,
    tbody,
  ]);
  const tableContainer = createElement(
    "div",
    {
      style:
        "position:absolute;left:-10000px;top:auto;width:1px;height:1px;overflow:hidden;",
    },
    [table]
  );

  const init = (u) => {
    u.root.setAttribute("aria-hidden", "true");
    u.root.parentElement.append(tableContainer);
  };

  const setData = (u) => {
    while (thead.firstChild) {
      tbody.removeChild(tbody.firstChild);
    }

    while (tbody.firstChild) {
      tbody.removeChild(tbody.firstChild);
    }

    const rows = u.series.map((series) =>
      createElement("tr", {}, [
        createElement("th", {
          scope: "row",
          textContent: series.label,
        }),
      ])
    );
    thead.append(rows[0]);
    tbody.append(...rows.slice(1));

    for (let idx = 0; idx <= u.data[0].length; idx++) {
      const isAllNull = u.data
        .slice(1)
        .every((s) => s[idx] == null || s[idx] === 0);
      if (!isAllNull) {
        rows[0].appendChild(
          createElement("th", {
            scope: "col",
            textContent: u.series[0].value(u, u.data[0][idx], 0, idx),
          })
        );
        for (let seriesIdx = 1; seriesIdx < rows.length; seriesIdx++) {
          rows[seriesIdx].appendChild(
            createElement("td", {
              textContent: u.series[seriesIdx].value(
                u,
                u.data[seriesIdx][idx],
                seriesIdx,
                idx
              ),
            })
          );
        }
      }
    }
  };

  const destroy = (_u) => {
    tableContainer.remove();
  };

  return {
    hooks: {
      init,
      setData,
      destroy,
    },
  };
};

const statPanel = ({
  title,
  statQuery,
  backgroundQuery,
  valueTransform,
  valueFormat,
  elementSelector,
}) => {
  const element = document.querySelector(elementSelector);

  element.appendChild(
    createElement("h2", {
      textContent: title,
    })
  );

  const getSize = () => getUPlotSize(element, 100);
  const opts = {
    ...getSize(),
    legend: { show: false },
    cursor: { show: false },
    series: [
      {},
      {
        spanGaps: true,
        stroke: color(0),
        fill: color(0, 0.1),
      },
    ],
    scales: { x: { time: false } },
    axes: [{ show: false }, { show: false }],
  };
  const plot = new uPlot(opts, emptyData(), element);
  onResize(() => plot.setSize(getSize()));

  const statEl = createElement("div", {
    className: "single-stat",
    textContent: valueFormat(emptyData()[1][0]),
  });
  element.appendChild(statEl);

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

const graphPanel = ({
  title,
  height,
  query,
  valueTransform,
  valueFormat,
  labelTag,
  elementSelector,
}) => {
  const element = document.querySelector(elementSelector);

  const headingId = `panel-headline-${title
    .toLowerCase()
    .replace(/[^a-z]/g, "-")}`;
  element.appendChild(
    createElement("h2", {
      id: headingId,
      textContent: title,
    })
  );

  const getSize = () => getUPlotSize(element, height);

  let plot;
  const recreatePlot = (raw, data) => {
    if (plot) {
      plot.destroy();
    }

    const opts = {
      ...getSize(),
      plugins: [accessibilityPlugin({ ariaLabelledBy: headingId })],
      series: [
        {},
        ...raw.map((series, i) => ({
          label: series.tags[labelTag],
          value: (_self, rawValue) => valueFormat(rawValue),
          stroke: color(i),
        })),
      ],
      axes: [
        {},
        {
          values: (_self, ticks) => ticks.map(valueFormat),
        },
      ],
    };
    plot = new uPlot(opts, data, element);
  };

  recreatePlot([], emptyData());
  onResize(() => plot.setSize(getSize()));

  const loadData = async () => {
    const raw = await queryData(query);
    const data = prepareData(raw, valueTransform);
    recreatePlot(raw, data);
  };
  loadData();
  onTimeRangeChange(loadData);
};

const tablePanel = ({
  title,
  query,
  values,
  labelTag,
  labelColumnName,
  elementSelector,
}) => {
  const element = document.querySelector(elementSelector);

  const headingId = `panel-headline-${title
    .toLowerCase()
    .replace(/[^a-z]/g, "-")}`;
  element.appendChild(
    createElement("h2", {
      id: headingId,
      textContent: title,
    })
  );

  const trHead = createElement("tr", {}, [
    createElement("th", {
      scope: "col",
      textContent: labelColumnName,
    }),
    ...values.map((value) =>
      createElement("th", {
        scope: "col",
        textContent: value.columnName,
      })
    ),
  ]);
  const thead = createElement("thead", {}, [trHead]);

  const tbody = createElement("tbody");

  const table = createElement(
    "table",
    {
      "aria-labelledby": headingId,
      className: "table-stat",
    },
    [thead, tbody]
  );

  element.append(table);

  const loadData = async () => {
    const raw = await queryData(query);

    while (tbody.firstChild) {
      tbody.removeChild(tbody.firstChild);
    }

    tbody.append(
      ...raw.map((series, i) =>
        createElement("tr", {}, [
          createElement("th", {
            scope: "row",
            textContent: series.tags[labelTag],
          }),
          ...values.map((value) =>
            createElement("td", {
              textContent: value.format(
                value.transform(
                  series.values[0][series.columns.indexOf(value.name)]
                )
              ),
            })
          ),
        ])
      )
    );
  };
  loadData();
  onTimeRangeChange(loadData);
};

window.addEventListener("load", () => {
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
    valueTransform: (value) => value * 100,
    valueFormat: (value) => `${formatNumber(value)}%`,
    elementSelector: "#overall-success",
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
    valueTransform: (value) => value / 1000 / 60,
    valueFormat: (value) => `${formatNumber(value)} min`,
    elementSelector: "#overall-duration",
  });

  tablePanel({
    title: "Statistics by pipeline",
    query: `
      SELECT count("commit") AS "count", mean("duration_ms") AS "duration_ms", mean("successful") AS "successful"
      FROM "build"
      WHERE __time_filter__
      GROUP BY "name"
    `,
    values: [
      {
        name: "count",
        columnName: "Count",
        transform: (value) => value,
        format: (value) => value,
      },
      {
        name: "duration_ms",
        columnName: "Duration",
        transform: (value) => value / 1000 / 60,
        format: (value) => `${formatNumber(value)} min`,
      },
      {
        name: "successful",
        columnName: "Success",
        transform: (value) => value * 100,
        format: (value) => `${formatNumber(value)}%`,
      },
    ],
    labelTag: "name",
    labelColumnName: "Pipeline",
    elementSelector: "#stats-by-pipeline",
  });

  graphPanel({
    title: "Duration",
    height: 410,
    query: `
      SELECT mean("duration_ms") AS value
      FROM "build"
      WHERE __time_filter__
      GROUP BY __time_group_detailed__, "name"
    `,
    valueTransform: (value) => value / 1000 / 60,
    valueFormat: (value) => `${formatNumber(value)} min`,
    labelTag: "name",
    elementSelector: "#duration",
  });

  graphPanel({
    title: "Attempts",
    height: 220,
    query: `
      SELECT mean("builds") AS value
      FROM "commit"
      WHERE __time_filter__
      GROUP BY __time_group_detailed__, "build_name"
    `,
    valueTransform: (value) => (value == null ? 0 : value),
    valueFormat: (value) => formatNumber(value),
    labelTag: "build_name",
    elementSelector: "#attempts",
  });
});
