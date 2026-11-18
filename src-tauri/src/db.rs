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
];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", i as i64 + 1)?;
    }
    Ok(())
}

#[tauri::command]
pub fn app_info(db: tauri::State<Db>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let v: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(format!("schema v{v}"))
}
