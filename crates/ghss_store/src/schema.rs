use rusqlite::{Connection, Result};

pub fn up(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
        CREATE TABLE IF NOT EXISTS builds (
            timestamp   INTEGER NOT NULL,
            name        TEXT NOT NULL,
            source      INTEGER NOT NULL,
            commit      TEXT NOT NULL,
            successful  INTEGER NOT NULL,
            failed      INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            PRIMARY KEY(timestamp, name, source)
        ) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS commits (
            timestamp         INTEGER NOT NULL,
            build_name        TEXT NOT NULL,
            build_source      INTEGER NOT NULL,
            commit            TEXT NOT NULL,
            builds            INTEGER NOT NULL,
            builds_successful INTEGER NOT NULL,
            builds_failed     INTEGER NOT NULL,
            PRIMARY KEY(timestamp, name, source)
        ) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS imports (
            timestamp INTEGER PRIMARY KEY,
            points    INTEGER NOT NULL,
        ) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS hooks (
            timestamp INTEGER NOT NULL,
            type      INTEGER NOT NULL,
            commit    TEXT NOT NULL,
            PRIMARY KEY(timestamp, type)
        ) WITHOUT ROWID;
        COMMIT;",
    )
}
