use rusqlite::{Connection, Result};

pub fn up(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
        CREATE TABLE builds (
            timestamp   INTEGER NOT NULL,
            name        TEXT NOT NULL,
            source      INTEGER NOT NULL,
            commit      TEXT NOT NULL,
            successful  INTEGER NOT NULL,
            failed      INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            PRIMARY KEY(timestamp, name, source)
        ) WITHOUT ROWID;
        CREATE TABLE commits (
            timestamp         INTEGER NOT NULL,
            build_nam         TEXT NOT NULL,
            build_source      INTEGER NOT NULL,
            commit            TEXT NOT NULL,
            builds            INTEGER NOT NULL,
            builds_successful INTEGER NOT NULL,
            builds_failed     INTEGER NOT NULL,
            PRIMARY KEY(timestamp, name, source)
        ) WITHOUT ROWID;
        CREATE TABLE imports (
            timestamp INTEGER PRIMARY KEY,
            points    INTEGER NOT NULL,
        ) WITHOUT ROWID;
        CREATE TABLE hooks (
            timestamp INTEGER NOT NULL,
            type      INTEGER NOT NULL,
            commit    TEXT NOT NULL,
            PRIMARY KEY(timestamp, type)
        ) WITHOUT ROWID;
        COMMIT;",
    )
}
