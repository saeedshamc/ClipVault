//! Forward-only SQLite migrations.

use rusqlite::{params, Connection};

use crate::error::{CoreError, CoreResult};

/// Current schema version applied by this build.
pub const SCHEMA_VERSION: i32 = 1;

/// Apply all pending migrations. Safe to call on every open.
pub fn migrate(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL
        );
        "#,
    )?;

    let current = current_version(conn)?;
    if current < 1 {
        migrate_v1(conn)?;
        record_version(conn, 1)?;
    }

    let current = current_version(conn)?;
    if current != SCHEMA_VERSION {
        return Err(CoreError::Storage(format!(
            "unsupported schema_version {current}; this build expects {SCHEMA_VERSION}"
        )));
    }

    Ok(())
}

fn current_version(conn: &Connection) -> CoreResult<i32> {
    let version: Option<i32> =
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    Ok(version.unwrap_or(0))
}

fn record_version(conn: &Connection, version: i32) -> CoreResult<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))",
        params![version],
    )?;
    Ok(())
}

fn migrate_v1(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_hash TEXT NOT NULL UNIQUE,
            kind TEXT NOT NULL,
            created_at TEXT NOT NULL,
            last_used_at TEXT NOT NULL,
            pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
            total_size INTEGER NOT NULL CHECK (total_size >= 0),
            source_app TEXT,
            searchable_text TEXT NOT NULL DEFAULT ''
        );

        CREATE INDEX idx_items_last_used_at ON items(last_used_at DESC);
        CREATE INDEX idx_items_pinned ON items(pinned);

        CREATE TABLE formats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            mime TEXT NOT NULL,
            size INTEGER NOT NULL CHECK (size >= 0),
            inline_data BLOB,
            blob_path TEXT,
            UNIQUE(item_id, mime)
        );

        CREATE INDEX idx_formats_item_id ON formats(item_id);

        CREATE VIRTUAL TABLE items_fts USING fts5(
            searchable_text,
            content='items',
            content_rowid='id'
        );

        CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
            INSERT INTO items_fts(rowid, searchable_text)
            VALUES (new.id, new.searchable_text);
        END;

        CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
            INSERT INTO items_fts(items_fts, rowid, searchable_text)
            VALUES ('delete', old.id, old.searchable_text);
        END;

        CREATE TRIGGER items_au AFTER UPDATE OF searchable_text ON items BEGIN
            INSERT INTO items_fts(items_fts, rowid, searchable_text)
            VALUES ('delete', old.id, old.searchable_text);
            INSERT INTO items_fts(rowid, searchable_text)
            VALUES (new.id, new.searchable_text);
        END;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().expect("open");
        migrate(&conn).expect("first");
        migrate(&conn).expect("second");
        assert_eq!(current_version(&conn).expect("version"), SCHEMA_VERSION);
    }
}
