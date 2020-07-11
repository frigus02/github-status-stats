use super::{Error, Result};
use crate::proto::{
    interval_aggregates_reply, total_aggregates_reply, AggregateFunction, Column, HookedCommit,
    IntervalAggregatesReply, IntervalType, TotalAggregatesReply,
};
use ghss_tracing::log_event;
use rusqlite::{params, Connection, OpenFlags};

pub struct DB {
    conn: Connection,
}

fn validate_identifier(s: &str) -> Result<()> {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(Error::InvalidIdentifier(s.into()))
    }
}

fn create_group_by(group_by_columns: Vec<String>) -> Result<Vec<String>> {
    group_by_columns
        .iter()
        .map(|c| {
            validate_identifier(&c)?;
            Ok(format!("\"{}\"", c))
        })
        .collect()
}

fn create_projection(columns: Vec<Column>, group_by: Vec<String>) -> Vec<String> {
    let mut projection = columns
        .iter()
        .map(|c| {
            let agg = match c.agg_func() {
                AggregateFunction::Avg => "avg",
                AggregateFunction::Count => "count",
            };
            format!("{}(\"{}\")", agg, c.name)
        })
        .collect::<Vec<_>>();
    if !group_by.is_empty() {
        projection.extend(group_by);
    }

    projection
}

fn create_aggregate_query_sql(
    projection: Vec<String>,
    table: String,
    from: i64,
    to: i64,
    group_by: Vec<String>,
    order_by: Option<&'static str>,
) -> String {
    let mut sql = format!(
        "SELECT {} FROM {} WHERE timestamp >= {} AND timestamp <= {}",
        projection.join(", "),
        table,
        from,
        to
    );
    if !group_by.is_empty() {
        sql.push_str(&format!(" GROUP BY {}", group_by.join(", ")));
    }
    if let Some(order_by) = order_by {
        sql.push_str(&format!(" ORDER BY {}", order_by));
    }

    sql
}

impl DB {
    pub fn open(directory: &str, repository_id: &str) -> Result<DB> {
        let path = format!("{}/{}.db", directory, repository_id);
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(DB { conn })
    }

    pub fn get_hooked_commits_since_last_import(&self, until: i64) -> Result<Vec<HookedCommit>> {
        let mut stmt = self.conn.prepare(
            "WITH last_import AS (SELECT timestamp FROM imports ORDER BY timestamp DESC LIMIT 1)
            SELECT \"commit\", group_concat(type) AS types
            FROM hooks
            WHERE timestamp > (SELECT timestamp FROM last_import) AND timestamp <= ?
            GROUP BY \"commit\"",
        )?;
        let hooked_commits = stmt
            .query_map(params![until], |row| {
                let commit = row.get(0)?;
                let types_comma_separated: String = row.get(1)?;
                let types = types_comma_separated
                    .split(',')
                    .map(|s| s.parse().unwrap())
                    .collect();
                Ok(HookedCommit { commit, types })
            })?
            .map(|row| row.map_err(|err| err.into()))
            .collect::<Result<_>>()?;
        Ok(hooked_commits)
    }

    pub fn get_total_aggregates(
        &self,
        table: String,
        columns: Vec<Column>,
        from: i64,
        to: i64,
        group_by_columns: Vec<String>,
    ) -> Result<TotalAggregatesReply> {
        validate_identifier(&table)?;
        if columns.is_empty() {
            return Err(Error::EmptyColumns);
        }

        let group_by = create_group_by(group_by_columns)?;

        let values_range = 0..columns.len();
        let groups_range = values_range.end..values_range.end + group_by.len();

        let projection = create_projection(columns, group_by.clone());
        let is_grouped = !group_by.is_empty();
        let sql = create_aggregate_query_sql(projection, table, from, to, group_by, None);

        log_event(format!("sql: {}", sql));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = if is_grouped {
            stmt.query_map(params![], |row| {
                Ok(total_aggregates_reply::Row {
                    values: values_range
                        .clone()
                        .map(|i| row.get(i))
                        .collect::<rusqlite::Result<_>>()?,
                    groups: groups_range
                        .clone()
                        .map(|i| row.get(i))
                        .collect::<rusqlite::Result<_>>()?,
                })
            })?
            .map(|row| row.map_err(|err| err.into()))
            .collect::<Result<_>>()?
        } else {
            // Without a GROUP BY clause SQLite always returns exactly 1 row.
            // If not rows match the WHERE clause, some aggregate functions
            // like avg() or max() return NULL.
            stmt.query_row(params![], |row| {
                let values: Vec<Option<f64>> = values_range
                    .clone()
                    .map(|i| row.get(i))
                    .collect::<rusqlite::Result<_>>()?;
                let rows = if values.iter().any(Option::is_none) {
                    Vec::new()
                } else {
                    vec![total_aggregates_reply::Row {
                        values: values.into_iter().map(Option::unwrap).collect(),
                        groups: Vec::new(),
                    }]
                };
                Ok(rows)
            })?
        };

        Ok(TotalAggregatesReply { rows })
    }

    pub fn get_interval_aggregates(
        &self,
        table: String,
        columns: Vec<Column>,
        from: i64,
        to: i64,
        group_by_columns: Vec<String>,
        interval_type: IntervalType,
    ) -> Result<IntervalAggregatesReply> {
        validate_identifier(&table)?;
        if columns.is_empty() {
            return Err(Error::EmptyColumns);
        }

        let mut group_by = create_group_by(group_by_columns)?;

        let values_range = 0..columns.len();
        let groups_range = values_range.end..values_range.end + group_by.len();
        let timestamp_index = groups_range.end;

        let mut projection = create_projection(columns, group_by.clone());

        group_by.push("interval".into());
        let time_range = to - from;
        if time_range <= 0 {
            return Err(Error::InvalidTimeRange);
        }
        let interval = match interval_type {
            IntervalType::Sparse => time_range / 120,
            IntervalType::Detailed => time_range / 720,
        };
        projection.push(format!(
            "timestamp / {} * {} AS interval",
            interval, interval
        ));

        let sql =
            create_aggregate_query_sql(projection, table, from, to, group_by, Some("interval"));

        log_event(format!("sql: {}", sql));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![], |row| {
                Ok(interval_aggregates_reply::Row {
                    values: values_range
                        .clone()
                        .map(|i| row.get(i))
                        .collect::<rusqlite::Result<_>>()?,
                    groups: groups_range
                        .clone()
                        .map(|i| row.get(i))
                        .collect::<rusqlite::Result<_>>()?,
                    timestamp: row.get(timestamp_index)?,
                })
            })?
            .map(|row| row.map_err(|err| err.into()))
            .collect::<Result<_>>()?;

        Ok(IntervalAggregatesReply { rows })
    }
}
