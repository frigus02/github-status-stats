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

const timeRange = () => {
  const start = startOfDay(startDateInput.valueAsNumber);
  const end = endOfDay(endDateInput.valueAsNumber);
  return { start, end };
};

const queryData = async ({ table, columns, groupBy, interval }) => {
  const time = timeRange();

  const url = new URL("/api/query", location);
  url.searchParams.append("repository", repository);
  url.searchParams.append("table", table);
  url.searchParams.append("columns", columns.join(","));
  url.searchParams.append("since", time.start);
  url.searchParams.append("until", time.end);
  if (groupBy && groupBy.length > 0) {
    url.searchParams.append("group_by", groupBy.join(","));
  }
  if (interval) {
    url.searchParams.append("interval", interval);
  }

  const res = await fetch(url.toString());
  if (!res.ok) {
    throw new Error(`Query failed eith ${res.status} ${res.statusText}`);
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
  if (raw.timestamps.length === 0) {
    return emptyData();
  }

  const data = [];
  data.push(raw.timestamps.map((t) => t / 1000));
  for (const series of raw.series) {
    data.push(
      series.values.map((v) => (v === null ? null : valueTransform(v[0])))
    );
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
    statEl.textContent = valueFormat(
      valueTransform(rawStat.series[0].values[0])
    );

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
        ...raw.series.map((series, i) => ({
          label: series.tags[0],
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

  recreatePlot({ series: [] }, emptyData());
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
      ...raw.series.map((series) =>
        createElement("tr", {}, [
          createElement("th", {
            scope: "row",
            textContent: series.tags[0],
          }),
          ...values.map((value, i) =>
            createElement("td", {
              textContent: value.format(value.transform(series.values[0][i])),
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
    statQuery: {
      table: "builds",
      columns: ["avg(successful)"],
    },
    backgroundQuery: {
      table: "builds",
      columns: ["avg(successful)"],
      interval: "sparse",
    },
    valueTransform: (value) => value * 100,
    valueFormat: (value) => `${formatNumber(value)}%`,
    elementSelector: "#overall-success",
  });

  statPanel({
    title: "Overall average duration",
    statQuery: {
      table: "builds",
      columns: ["avg(duration_ms)"],
    },
    backgroundQuery: {
      table: "builds",
      columns: ["avg(duration_ms)"],
      interval: "sparse",
    },
    valueTransform: (value) => value / 1000 / 60,
    valueFormat: (value) => `${formatNumber(value)} min`,
    elementSelector: "#overall-duration",
  });

  tablePanel({
    title: "Statistics by pipeline",
    query: {
      table: "builds",
      columns: ["count(commit)", "avg(duration_ms)", "avg(successful)"],
      groupBy: ["name"],
    },
    values: [
      {
        columnName: "Count",
        transform: (value) => value,
        format: (value) => value,
      },
      {
        columnName: "Duration",
        transform: (value) => value / 1000 / 60,
        format: (value) => `${formatNumber(value)} min`,
      },
      {
        columnName: "Success",
        transform: (value) => value * 100,
        format: (value) => `${formatNumber(value)}%`,
      },
    ],
    labelColumnName: "Pipeline",
    elementSelector: "#stats-by-pipeline",
  });

  graphPanel({
    title: "Duration",
    height: 410,
    query: {
      table: "builds",
      columns: ["avg(duration_ms)"],
      groupBy: ["name"],
      interval: "detailed",
    },
    valueTransform: (value) => value / 1000 / 60,
    valueFormat: (value) => `${formatNumber(value)} min`,
    elementSelector: "#duration",
  });

  graphPanel({
    title: "Attempts",
    height: 220,
    query: {
      table: "commits",
      columns: ["avg(builds)"],
      groupBy: ["build_name"],
      interval: "detailed",
    },
    valueTransform: (value) => (value == null ? 0 : value),
    valueFormat: (value) => formatNumber(value),
    elementSelector: "#attempts",
  });
});
