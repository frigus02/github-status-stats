use rusqlite::{Connection, Result};
use crate::schema;
use crate::proto::{Build, Commit, Import, Hook};

pub fn open(name: String) -> Result<Connection> {
    let conn = Connection::open(name)?;
    schema::up(conn)?;
    Ok(conn)
}

pub fn insert_builds(conn: &Connection, builds: &[Build]) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO builds(timestamp, name, source, commit, successful, failed, duration_ms)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(timestamp, name, source) DO UPDATE SET
            commit = excluded.commit,
            successful = excluded.successful,
            failed = excluded.failed,
            duration_ms = excluded.duration_ms",
    )?;
    for build in builds {
        stmt.execute(&[build.timestamp, build.name, build.source, build.commit, build.successful, build.failed, build.duration_ms])?;
    }
    Ok(())
}
