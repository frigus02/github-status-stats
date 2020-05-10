use super::Result;
use crate::proto::HookedCommit;
use rusqlite::{params, Connection, OpenFlags};

pub struct DB {
    conn: Connection,
}

impl DB {
    pub fn open(name: String) -> Result<DB> {
        let conn = Connection::open_with_flags(name, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
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
}
