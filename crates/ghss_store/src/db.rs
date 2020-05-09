use crate::proto::{Build, Commit, Hook};
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

    pub fn transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            transaction: self.conn.transaction()?,
        })
    }
}

pub struct Transaction<'conn> {
    transaction: rusqlite::Transaction<'conn>,
}

impl Transaction<'_> {
    pub fn upsert_builds(&self, builds: &[Build]) -> Result<()> {
        let mut stmt = self.transaction.prepare(
            "INSERT INTO builds(\"commit\", name, source, timestamp, successful, failed, duration_ms)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(\"commit\", name, source, timestamp) DO UPDATE SET
                successful = excluded.successful,
                failed = excluded.failed,
                duration_ms = excluded.duration_ms",
        )?;
        for build in builds {
            stmt.execute(params![
                build.commit,
                build.name,
                build.source,
                build.timestamp,
                build.successful,
                build.failed,
                build.duration_ms
            ])?;
        }
        Ok(())
    }

    pub fn upsert_commits(&self, commits: &[Commit]) -> Result<()> {
        let mut stmt = self.transaction.prepare(
            "INSERT INTO commits(\"commit\", build_name, build_source, builds, builds_successful, builds_failed, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(\"commit\", build_name, build_source) DO UPDATE SET
                builds = excluded.builds,
                builds_successful = excluded.builds_successful,
                builds_failed = excluded.builds_failed,
                timestamp = excluded.timestamp",
        )?;
        for commit in commits {
            stmt.execute(params![
                commit.commit,
                commit.build_name,
                commit.build_source,
                commit.builds,
                commit.builds_successful,
                commit.builds_failed,
                commit.timestamp
            ])?;
        }
        Ok(())
    }

    pub fn insert_import(&self, timestamp: i64, points: i32) -> Result<()> {
        let mut stmt = self.transaction.prepare(
            "INSERT INTO imports(timestamp, points)
            VALUES (?, ?)",
        )?;
        stmt.execute(params![timestamp, points])?;
        Ok(())
    }

    pub fn insert_hook(&self, hook: &Hook) -> Result<()> {
        let mut stmt = self.transaction.prepare(
            "INSERT INTO hooks(timestamp, type, \"commit\")
            VALUES (?, ?, ?)",
        )?;
        stmt.execute(params![hook.timestamp, hook.r#type, hook.commit])?;
        Ok(())
    }

    pub fn commit(self) -> Result<()> {
        self.transaction.commit()?;
        Ok(())
    }
}
