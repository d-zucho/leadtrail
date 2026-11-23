use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Shared SQLite connection, managed as Tauri state.
pub struct Db(pub Mutex<Connection>);

/// Ordered migrations. Append only; never edit a shipped entry.
const MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE cases (
        id INTEGER PRIMARY KEY,
        code TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        summary TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'investigating',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
"#,
    r#"
    CREATE TABLE sources (
        id INTEGER PRIMARY KEY,
        case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        url TEXT,
        notes TEXT NOT NULL DEFAULT '',
        file_path TEXT,
        file_name TEXT,
        size INTEGER,
        hash TEXT,
        verification_status TEXT NOT NULL DEFAULT 'unverified',
        verification_note TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX sources_case ON sources(case_id);
    CREATE UNIQUE INDEX sources_case_hash ON sources(case_id, hash) WHERE hash IS NOT NULL;
"#,
    r#"
    CREATE TABLE leads (
        id INTEGER PRIMARY KEY,
        title TEXT NOT NULL,
        kind TEXT NOT NULL DEFAULT 'note',
        url TEXT,
        body TEXT NOT NULL DEFAULT '',
        case_id INTEGER REFERENCES cases(id) ON DELETE SET NULL,
        status TEXT NOT NULL DEFAULT 'new',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX leads_status ON leads(status);
"#,
    r#"
    CREATE TABLE documents (
        id INTEGER PRIMARY KEY,
        case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
        kind TEXT NOT NULL DEFAULT 'note',
        title TEXT NOT NULL DEFAULT '',
        content_json TEXT NOT NULL DEFAULT '',
        content_text TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX documents_case ON documents(case_id);
    CREATE TABLE links (
        id INTEGER PRIMARY KEY,
        src_doc_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
        dst_type TEXT NOT NULL,
        dst_id INTEGER NOT NULL,
        label TEXT NOT NULL DEFAULT '',
        UNIQUE (src_doc_id, dst_type, dst_id)
    );
    CREATE INDEX links_dst ON links(dst_type, dst_id);
"#,
    r#"
    CREATE TABLE questions (
        id INTEGER PRIMARY KEY,
        case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
        text TEXT NOT NULL,
        due TEXT,
        done INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX questions_case ON questions(case_id);
"#,
    r#"
    CREATE TABLE entities (
        id INTEGER PRIMARY KEY,
        case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        name TEXT NOT NULL,
        notes TEXT NOT NULL DEFAULT '',
        x REAL,
        y REAL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE UNIQUE INDEX entities_unique ON entities(case_id, kind, name COLLATE NOCASE);

    CREATE TABLE relationships (
        id INTEGER PRIMARY KEY,
        case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
        from_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
        to_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
        label TEXT NOT NULL,
        notes TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        CHECK (from_id <> to_id)
    );
    CREATE UNIQUE INDEX relationships_unique ON relationships(from_id, to_id, label COLLATE NOCASE);
    CREATE INDEX relationships_case ON relationships(case_id);

    CREATE TABLE relationship_sources (
        relationship_id INTEGER NOT NULL REFERENCES relationships(id) ON DELETE CASCADE,
        source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
        quote TEXT NOT NULL DEFAULT '',
        locator TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        PRIMARY KEY (relationship_id, source_id)
    );
    CREATE INDEX relationship_sources_source ON relationship_sources(source_id);
"#,
    r#"
    CREATE VIRTUAL TABLE search USING fts5(
        title, body,
        kind UNINDEXED, ref_id UNINDEXED, case_id UNINDEXED,
        tokenize = 'unicode61 remove_diacritics 2'
    );
    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 1, cases.title, cases.summary, 'case', id, cases.id FROM cases;
    CREATE TRIGGER search_cases_ai AFTER INSERT ON cases BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 1, new.title, new.summary, 'case', new.id, new.id); END;
    CREATE TRIGGER search_cases_au AFTER UPDATE ON cases BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 1; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 1, new.title, new.summary, 'case', new.id, new.id); END;
    CREATE TRIGGER search_cases_ad AFTER DELETE ON cases BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 1; END;
    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 2, sources.title, sources.notes || ' ' || coalesce(sources.url,'') || ' ' || coalesce(sources.file_name,''), 'source', id, sources.case_id FROM sources;
    CREATE TRIGGER search_sources_ai AFTER INSERT ON sources BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 2, new.title, new.notes || ' ' || coalesce(new.url,'') || ' ' || coalesce(new.file_name,''), 'source', new.id, new.case_id); END;
    CREATE TRIGGER search_sources_au AFTER UPDATE ON sources BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 2; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 2, new.title, new.notes || ' ' || coalesce(new.url,'') || ' ' || coalesce(new.file_name,''), 'source', new.id, new.case_id); END;
    CREATE TRIGGER search_sources_ad AFTER DELETE ON sources BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 2; END;
    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 3, leads.title, leads.body || ' ' || coalesce(leads.url,''), 'lead', id, leads.case_id FROM leads;
    CREATE TRIGGER search_leads_ai AFTER INSERT ON leads BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 3, new.title, new.body || ' ' || coalesce(new.url,''), 'lead', new.id, new.case_id); END;
    CREATE TRIGGER search_leads_au AFTER UPDATE ON leads BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 3; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 3, new.title, new.body || ' ' || coalesce(new.url,''), 'lead', new.id, new.case_id); END;
    CREATE TRIGGER search_leads_ad AFTER DELETE ON leads BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 3; END;
    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 4, documents.title, documents.content_text, 'document', id, documents.case_id FROM documents;
    CREATE TRIGGER search_documents_ai AFTER INSERT ON documents BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 4, new.title, new.content_text, 'document', new.id, new.case_id); END;
    CREATE TRIGGER search_documents_au AFTER UPDATE ON documents BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 4; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 4, new.title, new.content_text, 'document', new.id, new.case_id); END;
    CREATE TRIGGER search_documents_ad AFTER DELETE ON documents BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 4; END;
    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 5, questions.text, '', 'question', id, questions.case_id FROM questions;
    CREATE TRIGGER search_questions_ai AFTER INSERT ON questions BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 5, new.text, '', 'question', new.id, new.case_id); END;
    CREATE TRIGGER search_questions_au AFTER UPDATE ON questions BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 5; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 5, new.text, '', 'question', new.id, new.case_id); END;
    CREATE TRIGGER search_questions_ad AFTER DELETE ON questions BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 5; END;
    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 6, entities.name, entities.notes, 'entity', id, entities.case_id FROM entities;
    CREATE TRIGGER search_entities_ai AFTER INSERT ON entities BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 6, new.name, new.notes, 'entity', new.id, new.case_id); END;
    CREATE TRIGGER search_entities_au AFTER UPDATE ON entities BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 6; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 6, new.name, new.notes, 'entity', new.id, new.case_id); END;
    CREATE TRIGGER search_entities_ad AFTER DELETE ON entities BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 6; END;
"#,
    r#"
    ALTER TABLE documents ADD COLUMN status TEXT NOT NULL DEFAULT 'draft';
    ALTER TABLE documents ADD COLUMN published_at TEXT;
    CREATE INDEX documents_status ON documents(kind, status);
"#,
    r#"
    CREATE TABLE parties (
        id INTEGER PRIMARY KEY,
        kind TEXT NOT NULL,
        name TEXT NOT NULL,
        aliases TEXT NOT NULL DEFAULT '',
        description TEXT NOT NULL DEFAULT '',
        shared INTEGER NOT NULL DEFAULT 0 CHECK (shared IN (0,1)),
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    INSERT INTO parties (id, kind, name) SELECT id, kind, name FROM entities;

    ALTER TABLE entities ADD COLUMN party_id INTEGER REFERENCES parties(id);
    UPDATE entities SET party_id = id;
    DROP INDEX entities_unique;
    CREATE UNIQUE INDEX entities_case_party ON entities(case_id, party_id);
    CREATE INDEX entities_party ON entities(party_id);

    CREATE TRIGGER entities_require_party BEFORE INSERT ON entities WHEN new.party_id IS NULL
    BEGIN SELECT RAISE(ABORT, 'an entity must reference a party'); END;
    CREATE TRIGGER parties_sync_entities AFTER UPDATE OF kind, name ON parties
    BEGIN UPDATE entities SET kind = new.kind, name = new.name WHERE party_id = new.id; END;

    INSERT INTO search(rowid, title, body, kind, ref_id, case_id)
        SELECT id * 8 + 7, name, aliases || ' ' || description, 'party', id, NULL FROM parties;
    CREATE TRIGGER search_parties_ai AFTER INSERT ON parties BEGIN INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 7, new.name, new.aliases || ' ' || new.description, 'party', new.id, NULL); END;
    CREATE TRIGGER search_parties_au AFTER UPDATE ON parties BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 7; INSERT INTO search(rowid, title, body, kind, ref_id, case_id) VALUES (new.id * 8 + 7, new.name, new.aliases || ' ' || new.description, 'party', new.id, NULL); END;
    CREATE TRIGGER search_parties_ad AFTER DELETE ON parties BEGIN DELETE FROM search WHERE rowid = old.id * 8 + 7; END;
"#,
];

/// The schema version this build understands.
pub fn schema_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    migrate_to(conn, MIGRATIONS.len())
}

/// Applies pending migrations up to and including `target` (a schema version).
pub(crate) fn migrate_to(conn: &Connection, target: usize) -> rusqlite::Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().take(target).skip(current as usize) {
        apply(conn, sql, i as i64 + 1)?;
    }
    Ok(())
}

/// Runs one migration and its version bump atomically: a failure leaves the DB at the previous version.
fn apply(conn: &Connection, sql: &str, version: i64) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(sql)?;
    tx.pragma_update(None, "user_version", version)?;
    tx.commit()
}

#[tauri::command]
pub fn app_info(db: tauri::State<Db>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let v: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(format!("schema v{v}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v8() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate_to(&conn, 8).unwrap();
        conn
    }

    #[test]
    fn a_failing_migration_rolls_back_completely() {
        let conn = v8();
        let r = apply(&conn, "CREATE TABLE half_done (id INTEGER); INSERT INTO no_such_table VALUES (1);", 9);
        assert!(r.is_err());
        assert_eq!(count(&conn, "PRAGMA user_version"), 8);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sqlite_master WHERE name = 'half_done'"), 0);
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn v9_backfills_one_party_per_entity_and_keeps_the_graph() {
        let conn = v8();
        conn.execute_batch(
            "INSERT INTO cases (id, code, title) VALUES (1, 'A', 'Case A');
             INSERT INTO entities (id, case_id, kind, name, x, y) VALUES
                (1, 1, 'organization', 'Northbank', 10.5, 20.5),
                (2, 1, 'public_body', 'Water dept', NULL, NULL),
                (3, 1, 'person', 'Maya', NULL, NULL);
             INSERT INTO sources (id, case_id, kind, title) VALUES (10, 1, 'record', 'Award notice');
             INSERT INTO relationships (id, case_id, from_id, to_id, label) VALUES (1, 1, 1, 2, 'funds');
             INSERT INTO relationship_sources (relationship_id, source_id) VALUES (1, 10);",
        )
        .unwrap();
        migrate_to(&conn, 9).unwrap();

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM parties"), 3);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM entities WHERE party_id = id"), 3);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM entities e JOIN parties p ON p.id = e.party_id WHERE p.name = e.name AND p.kind = e.kind"),
            3
        );
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM parties WHERE shared = 0"), 3);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM relationships"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM relationship_sources"), 1);
        let (x, y): (f64, f64) = conn.query_row("SELECT x, y FROM entities WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((x, y), (10.5, 20.5));
        assert_eq!(count(&conn, "PRAGMA user_version"), 9);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM search WHERE search MATCH '\"northbank\"*' AND kind='party'"), 1);
    }

    #[test]
    fn renaming_a_party_updates_entities_and_search() {
        let conn = v8();
        conn.execute_batch(
            "INSERT INTO cases (id, code, title) VALUES (1, 'A', 'Case A');
             INSERT INTO entities (id, case_id, kind, name) VALUES (1, 1, 'organization', 'Northbank');",
        )
        .unwrap();
        migrate_to(&conn, 9).unwrap();
        conn.execute("UPDATE parties SET name='Renamed', kind='person' WHERE id=1", []).unwrap();

        let (k, n): (String, String) = conn
            .query_row("SELECT kind, name FROM entities WHERE party_id = 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((k.as_str(), n.as_str()), ("person", "Renamed"));
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM search WHERE search MATCH '\"renamed\"*' AND kind='entity'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM search WHERE search MATCH '\"renamed\"*' AND kind='party'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM search WHERE search MATCH '\"northbank\"*'"), 0);
    }

    #[test]
    fn entity_insert_requires_a_party() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&conn).unwrap();
        conn.execute("INSERT INTO cases (id, code, title) VALUES (1, 'A', 'Case A')", []).unwrap();
        assert!(conn.execute("INSERT INTO entities (case_id, kind, name) VALUES (1,'person','X')", []).is_err());
        conn.execute("INSERT INTO parties (id, kind, name) VALUES (1, 'person', 'X')", []).unwrap();
        conn.execute("INSERT INTO entities (case_id, kind, name, party_id) VALUES (1,'person','X',1)", []).unwrap();
    }

    #[test]
    fn two_parties_may_share_a_name_in_one_case_but_one_party_links_once() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO cases (id, code, title) VALUES (1, 'A', 'Case A');
             INSERT INTO parties (id, kind, name) VALUES (1, 'person', 'John Smith'), (2, 'person', 'John Smith');",
        )
        .unwrap();
        for pid in [1, 2] {
            conn.execute("INSERT INTO entities (case_id, kind, name, party_id) VALUES (1,'person','John Smith',?1)", [pid]).unwrap();
        }
        assert!(conn
            .execute("INSERT INTO entities (case_id, kind, name, party_id) VALUES (1,'person','John Smith',1)", [])
            .is_err());
    }
}
