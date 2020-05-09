use crate::proto::{Build, Commit, Hook, Import};
use crate::schema;
use rusqlite::{params, Connection};
use std::convert::From;

pub enum Error {
    SQLite(rusqlite::Error),
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::SQLite(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct DB {
    conn: Connection,
}

impl DB {
    pub fn open(name: String) -> Result<DB> {
        let conn = Connection::open(name)?;
        schema::up(&conn)?;
        Ok(DB { conn })
    }

    pub fn insert_builds(&self, builds: &[Build]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO builds(timestamp, name, source, commit, successful, failed, duration_ms)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(timestamp, name, source) DO UPDATE SET
                commit = excluded.commit,
                successful = excluded.successful,
                failed = excluded.failed,
                duration_ms = excluded.duration_ms",
        )?;
        for build in builds {
            stmt.execute(params![
                build.timestamp,
                build.name,
                build.source,
                build.commit,
                build.successful,
                build.failed,
                build.duration_ms
            ])?;
        }
        Ok(())
    }

    pub fn insert_commits(&self, commits: &[Commit]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO commits(timestamp, build_name, build_source, commit, builds, builds_successful, builds_failed)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(timestamp, build_name, build_source) DO UPDATE SET
                commit = excluded.commit,
                builds = excluded.builds,
                builds_successful = excluded.builds_successful,
                builds_failed = excluded.builds_failed",
        )?;
        for commit in commits {
            stmt.execute(params![
                commit.timestamp,
                commit.build_name,
                commit.build_source,
                commit.commit,
                commit.builds,
                commit.builds_successful,
                commit.builds_failed
            ])?;
        }
        Ok(())
    }

    pub fn insert_imports(&self, imports: &[Import]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO imports(timestamp, points)
            VALUES (?, ?)
            ON CONFLICT(timestamp) DO UPDATE SET
                points = excluded.points",
        )?;
        for import in imports {
            stmt.execute(params![import.timestamp, import.points])?;
        }
        Ok(())
    }

    pub fn insert_hooks(&self, hooks: &[Hook]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO hooks(timestamp, type, commit)
            VALUES (?, ?, ?)
            ON CONFLICT(timestamp, type) DO UPDATE SET
                commit = excluded.commit",
        )?;
        for hook in hooks {
            stmt.execute(params![hook.timestamp, hook.r#type, hook.commit])?;
        }
        Ok(())
    }
}
