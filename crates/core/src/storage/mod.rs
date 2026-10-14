//! SQLite-backed clipboard history storage.

mod migrations;

pub use migrations::{migrate, SCHEMA_VERSION};

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::{CoreError, CoreResult};
use crate::model::{ClipboardItem, FormatPayload, ItemKind, NewClipboardItem};

/// Open a database at `path`, enable WAL, and run migrations.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (or create) the database file and prepare schema.
    pub fn open(path: &Path) -> CoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| CoreError::Path {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// In-memory store for unit tests.
    pub fn open_in_memory() -> CoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> CoreResult<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Reported schema version from the migrations table.
    pub fn schema_version(&self) -> CoreResult<i32> {
        let version: Option<i32> =
            self.conn
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get(0)
                })?;
        Ok(version.unwrap_or(0))
    }

    /// Insert a new item and its formats. Returns the new row id.
    pub fn insert(&self, new_item: &NewClipboardItem) -> CoreResult<i64> {
        new_item.validate().map_err(CoreError::InvalidInput)?;

        let now = Utc::now();
        let total_size = new_item.total_size() as i64;

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            r#"
            INSERT INTO items (
                content_hash, kind, created_at, last_used_at, pinned,
                total_size, source_app, searchable_text
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                new_item.content_hash,
                new_item.kind.as_str(),
                now.to_rfc3339(),
                now.to_rfc3339(),
                new_item.pinned as i32,
                total_size,
                new_item.source_app,
                new_item.searchable_text,
            ],
        )?;
        let item_id = tx.last_insert_rowid();
        insert_formats(&tx, item_id, &new_item.formats)?;
        tx.commit()?;
        Ok(item_id)
    }

    /// Fetch a single item with all formats.
    pub fn get(&self, id: i64) -> CoreResult<ClipboardItem> {
        let mut item = self.query_item(
            "SELECT id, content_hash, kind, created_at, last_used_at, pinned, total_size, source_app, searchable_text
             FROM items WHERE id = ?1",
            params![id],
        )?
        .ok_or(CoreError::NotFound(id))?;
        item.formats = self.load_formats(id)?;
        Ok(item)
    }

    /// Look up by content hash.
    pub fn get_by_hash(&self, content_hash: &str) -> CoreResult<Option<ClipboardItem>> {
        let mut item = self.query_item(
            "SELECT id, content_hash, kind, created_at, last_used_at, pinned, total_size, source_app, searchable_text
             FROM items WHERE content_hash = ?1",
            params![content_hash],
        )?;
        if let Some(ref mut item) = item {
            let id = item.id.expect("id present");
            item.formats = self.load_formats(id)?;
        }
        Ok(item)
    }

    /// List items newest-first (`last_used_at` descending).
    pub fn list(&self, limit: u32, offset: u32) -> CoreResult<Vec<ClipboardItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content_hash, kind, created_at, last_used_at, pinned, total_size, source_app, searchable_text
             FROM items
             ORDER BY last_used_at DESC, id DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], map_item_row)?;
        let mut items = Vec::new();
        for row in rows {
            let mut item = row?;
            let id = item.id.expect("id present");
            item.formats = self.load_formats(id)?;
            items.push(item);
        }
        Ok(items)
    }

    /// Full-text search via FTS5.
    ///
    /// User input is tokenized into alphanumeric terms joined with `AND` so
    /// punctuation (e.g. hyphens) cannot be interpreted as FTS operators.
    pub fn search(&self, query: &str, limit: u32) -> CoreResult<Vec<ClipboardItem>> {
        let Some(fts_query) = build_fts_query(query) else {
            return self.list(limit, 0);
        };

        let mut stmt = self.conn.prepare(
            "SELECT items.id, items.content_hash, items.kind, items.created_at, items.last_used_at,
                    items.pinned, items.total_size, items.source_app, items.searchable_text
             FROM items_fts
             JOIN items ON items.id = items_fts.rowid
             WHERE items_fts MATCH ?1
             ORDER BY items.last_used_at DESC, items.id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts_query, limit], map_item_row)?;
        let mut items = Vec::new();
        for row in rows {
            let mut item = row?;
            let id = item.id.expect("id present");
            item.formats = self.load_formats(id)?;
            items.push(item);
        }
        Ok(items)
    }

    /// Update pin flag.
    pub fn set_pinned(&self, id: i64, pinned: bool) -> CoreResult<()> {
        let changed = self.conn.execute(
            "UPDATE items SET pinned = ?1 WHERE id = ?2",
            params![pinned as i32, id],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    /// Touch `last_used_at` (used by dedupe/reuse later).
    pub fn touch(&self, id: i64, when: DateTime<Utc>) -> CoreResult<()> {
        let changed = self.conn.execute(
            "UPDATE items SET last_used_at = ?1 WHERE id = ?2",
            params![when.to_rfc3339(), id],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    /// Delete one item (formats cascade).
    pub fn delete(&self, id: i64) -> CoreResult<()> {
        let changed = self
            .conn
            .execute("DELETE FROM items WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    /// Delete every history item. Returns how many rows were removed.
    pub fn clear(&self) -> CoreResult<usize> {
        let changed = self.conn.execute("DELETE FROM items", [])?;
        Ok(changed)
    }

    /// Count stored items.
    pub fn count(&self) -> CoreResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Sum of `total_size` across all items.
    pub fn total_size(&self) -> CoreResult<u64> {
        let sum: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_size), 0) FROM items",
            [],
            |row| row.get(0),
        )?;
        Ok(sum as u64)
    }

    fn query_item(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> CoreResult<Option<ClipboardItem>> {
        let mut stmt = self.conn.prepare(sql)?;
        let item = stmt.query_row(params, map_item_row).optional()?;
        Ok(item)
    }

    fn load_formats(&self, item_id: i64) -> CoreResult<Vec<FormatPayload>> {
        let mut stmt = self.conn.prepare(
            "SELECT mime, size, inline_data, blob_path FROM formats WHERE item_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![item_id], |row| {
            Ok(FormatPayload {
                mime: row.get(0)?,
                size: row.get::<_, i64>(1)? as u64,
                inline_data: row.get(2)?,
                blob_path: row.get(3)?,
            })
        })?;
        let mut formats = Vec::new();
        for row in rows {
            formats.push(row?);
        }
        Ok(formats)
    }
}

/// Build a safe FTS5 MATCH expression from free-form user text.
fn build_fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{token}\""))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

fn insert_formats(tx: &Transaction<'_>, item_id: i64, formats: &[FormatPayload]) -> CoreResult<()> {
    for format in formats {
        tx.execute(
            r#"
            INSERT INTO formats (item_id, mime, size, inline_data, blob_path)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                item_id,
                format.mime,
                format.size as i64,
                format.inline_data,
                format.blob_path,
            ],
        )?;
    }
    Ok(())
}

fn map_item_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardItem> {
    let kind_str: String = row.get(2)?;
    let kind = ItemKind::parse(&kind_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(CoreError::Storage(format!("unknown item kind: {kind_str}"))),
        )
    })?;
    let created_at = parse_time(row.get::<_, String>(3)?)?;
    let last_used_at = parse_time(row.get::<_, String>(4)?)?;
    let pinned_i: i32 = row.get(5)?;
    Ok(ClipboardItem {
        id: Some(row.get(0)?),
        content_hash: row.get(1)?,
        kind,
        created_at,
        last_used_at,
        pinned: pinned_i != 0,
        total_size: row.get::<_, i64>(6)? as u64,
        source_app: row.get(7)?,
        searchable_text: row.get(8)?,
        formats: Vec::new(),
    })
}

fn parse_time(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FormatPayload, ItemKind, NewClipboardItem};

    fn sample_text(hash: &str, text: &str) -> NewClipboardItem {
        NewClipboardItem {
            content_hash: hash.into(),
            kind: ItemKind::Text,
            pinned: false,
            source_app: Some("test".into()),
            searchable_text: text.into(),
            formats: vec![FormatPayload::text_plain(text)],
        }
    }

    #[test]
    fn open_in_memory_sets_schema_version() {
        let store = Store::open_in_memory().expect("open");
        assert_eq!(store.schema_version().expect("version"), SCHEMA_VERSION);
    }

    #[test]
    fn insert_get_list_delete() {
        let store = Store::open_in_memory().expect("open");
        let id = store
            .insert(&sample_text("h1", "hello world"))
            .expect("insert");
        let item = store.get(id).expect("get");
        assert_eq!(item.searchable_text, "hello world");
        assert_eq!(item.formats.len(), 1);
        assert_eq!(item.formats[0].mime, "text/plain");

        let listed = store.list(10, 0).expect("list");
        assert_eq!(listed.len(), 1);

        store.delete(id).expect("delete");
        assert!(matches!(store.get(id), Err(CoreError::NotFound(_))));
        assert_eq!(store.count().expect("count"), 0);
    }

    #[test]
    fn multi_format_item_roundtrip() {
        let store = Store::open_in_memory().expect("open");
        let new_item = NewClipboardItem {
            content_hash: "multi".into(),
            kind: ItemKind::Html,
            pinned: false,
            source_app: None,
            searchable_text: "hello".into(),
            formats: vec![
                FormatPayload::text_plain("hello"),
                FormatPayload::text_html("<b>hello</b>"),
            ],
        };
        let id = store.insert(&new_item).expect("insert");
        let item = store.get(id).expect("get");
        assert_eq!(item.formats.len(), 2);
        assert_eq!(item.total_size, new_item.total_size());
    }

    #[test]
    fn search_finds_text() {
        let store = Store::open_in_memory().expect("open");
        store
            .insert(&sample_text("a", "alpha unique-token"))
            .expect("insert a");
        store
            .insert(&sample_text("b", "beta other"))
            .expect("insert b");

        let hits = store.search("unique-token", 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].content_hash, "a");

        let phrase = store.search("unique", 10).expect("search unique");
        assert_eq!(phrase.len(), 1);
    }

    #[test]
    fn build_fts_query_strips_operators() {
        assert_eq!(
            build_fts_query("unique-token"),
            Some("\"unique\" AND \"token\"".into())
        );
        assert_eq!(build_fts_query("   "), None);
    }

    #[test]
    fn clear_removes_all() {
        let store = Store::open_in_memory().expect("open");
        store.insert(&sample_text("a", "one")).expect("insert");
        store.insert(&sample_text("b", "two")).expect("insert");
        assert_eq!(store.clear().expect("clear"), 2);
        assert_eq!(store.count().expect("count"), 0);
    }

    #[test]
    fn duplicate_hash_is_rejected() {
        let store = Store::open_in_memory().expect("open");
        store.insert(&sample_text("dup", "one")).expect("insert");
        let err = store
            .insert(&sample_text("dup", "two"))
            .expect_err("duplicate");
        assert!(matches!(err, CoreError::Storage(_)));
    }

    #[test]
    fn pin_and_touch() {
        let store = Store::open_in_memory().expect("open");
        let id = store.insert(&sample_text("p", "pin me")).expect("insert");
        store.set_pinned(id, true).expect("pin");
        let item = store.get(id).expect("get");
        assert!(item.pinned);

        let later = Utc::now() + chrono::Duration::seconds(5);
        store.touch(id, later).expect("touch");
        let item = store.get(id).expect("get");
        assert_eq!(item.last_used_at, later);
    }

    #[test]
    fn file_backed_store_survives_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        {
            let store = Store::open(&path).expect("open");
            store
                .insert(&sample_text("persist", "durable"))
                .expect("insert");
        }
        let store = Store::open(&path).expect("reopen");
        let items = store.list(10, 0).expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].searchable_text, "durable");
    }
}
