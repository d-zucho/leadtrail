use crate::db::Db;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use tauri::State;

const KINDS: &[&str] = &["note", "article"];
pub const STATUSES: &[&str] = &["draft", "in_review", "ready", "published"];

/// Full document, including editor content.
#[derive(Serialize, Debug)]
pub struct Doc {
    pub id: i64,
    pub case_id: i64,
    pub kind: String,
    pub title: String,
    pub content_json: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Lightweight row for lists; never carries the full content.
#[derive(Serialize)]
pub struct DocSummary {
    pub id: i64,
    pub case_id: i64,
    pub kind: String,
    pub title: String,
    pub excerpt: String,
    pub status: String,
    pub updated_at: String,
}

/// An article in the publication queue, with the case it belongs to.
#[derive(Serialize)]
pub struct QueueItem {
    pub id: i64,
    pub case_id: i64,
    pub case_code: String,
    pub case_title: String,
    pub title: String,
    pub status: String,
    pub words: i64,
    pub published_at: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct Backlink {
    pub doc_id: i64,
    pub case_id: i64,
    pub kind: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct LinkRef {
    pub dst_type: String,
    pub dst_id: i64,
    pub label: String,
}

#[derive(Deserialize)]
pub struct DocSave {
    pub title: String,
    pub content_json: String,
    pub content_text: String,
    pub links: Vec<LinkRef>,
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn fetch(conn: &Connection, id: i64) -> Result<Doc, String> {
    let mut doc = conn.query_row(
        "SELECT id, case_id, kind, title, content_json, status, published_at, created_at, updated_at FROM documents WHERE id = ?1",
        [id],
        |r| {
            Ok(Doc {
                id: r.get(0)?,
                case_id: r.get(1)?,
                kind: r.get(2)?,
                title: r.get(3)?,
                content_json: r.get(4)?,
                status: r.get(5)?,
                published_at: r.get(6)?,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
            })
        },
    )
    .map_err(|_| "document not found".to_string())?;
    doc.content_json = crate::mentions::refresh_party_labels(conn, &doc.content_json);
    Ok(doc)
}

fn summary(r: &Row) -> rusqlite::Result<DocSummary> {
    Ok(DocSummary {
        id: r.get(0)?,
        case_id: r.get(1)?,
        kind: r.get(2)?,
        title: r.get(3)?,
        excerpt: r.get(4)?,
        status: r.get(5)?,
        updated_at: r.get(6)?,
    })
}

/// Replaces a document's outgoing links. Targets must exist in the same case, so the
/// link graph never holds dangling or cross-case references.
pub fn replace_links(
    conn: &Connection,
    doc_id: i64,
    case_id: i64,
    links: &[LinkRef],
) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM links WHERE src_doc_id = ?1", [doc_id])?;
    for l in links {
        let table = match l.dst_type.as_str() {
            "document" => "documents",
            "source" => "sources",
            "party" => "parties",
            _ => continue,
        };
        if l.dst_type == "document" && l.dst_id == doc_id {
            continue;
        }
        // Parties are global records, so they are not tied to the note's case.
        let ok = if l.dst_type == "party" {
            conn.query_row("SELECT 1 FROM parties WHERE id = ?1", [l.dst_id], |_| Ok(()))
        } else {
            conn.query_row(
                &format!("SELECT 1 FROM {table} WHERE id = ?1 AND case_id = ?2"),
                params![l.dst_id, case_id],
                |_| Ok(()),
            )
        }
        .is_ok();
        if ok {
            conn.execute(
                "INSERT OR IGNORE INTO links (src_doc_id, dst_type, dst_id, label) VALUES (?1, ?2, ?3, ?4)",
                params![doc_id, l.dst_type, l.dst_id, l.label],
            )?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn list_documents(db: State<Db>, case_id: i64) -> Result<Vec<DocSummary>, String> {
    let conn = db.0.lock().map_err(err)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, case_id, kind, title, substr(content_text, 1, 160), status, updated_at \
             FROM documents WHERE case_id = ?1 ORDER BY updated_at DESC, id DESC",
        )
        .map_err(err)?;
    let rows = stmt.query_map([case_id], summary).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

/// Moves an article through draft → in review → ready → published (any direction).
/// Notes are private working material and never enter the queue.
pub fn set_status(conn: &Connection, id: i64, status: &str) -> Result<Doc, String> {
    if !STATUSES.contains(&status) {
        return Err(format!("invalid status: {status}"));
    }
    let doc = fetch(conn, id)?;
    if doc.kind != "article" {
        return Err("only articles move through the publication queue".into());
    }
    conn.execute(
        "UPDATE documents SET status = ?1, \
         published_at = CASE WHEN ?1 = 'published' THEN coalesce(published_at, datetime('now')) ELSE NULL END \
         WHERE id = ?2",
        params![status, id],
    )
    .map_err(err)?;
    fetch(conn, id)
}

#[tauri::command]
pub fn set_document_status(db: State<Db>, id: i64, status: String) -> Result<Doc, String> {
    let conn = db.0.lock().map_err(err)?;
    set_status(&conn, id, &status)
}

pub fn queue(conn: &Connection) -> rusqlite::Result<Vec<QueueItem>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.case_id, c.code, c.title, d.title, d.status, d.content_text, d.published_at, d.updated_at \
         FROM documents d JOIN cases c ON c.id = d.case_id \
         WHERE d.kind = 'article' ORDER BY d.updated_at DESC, d.id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        let text: String = r.get(6)?;
        Ok(QueueItem {
            id: r.get(0)?,
            case_id: r.get(1)?,
            case_code: r.get(2)?,
            case_title: r.get(3)?,
            title: r.get(4)?,
            status: r.get(5)?,
            words: text.split_whitespace().count() as i64,
            published_at: r.get(7)?,
            updated_at: r.get(8)?,
        })
    })?;
    rows.collect()
}

#[tauri::command]
pub fn list_queue(db: State<Db>) -> Result<Vec<QueueItem>, String> {
    let conn = db.0.lock().map_err(err)?;
    queue(&conn).map_err(err)
}

#[tauri::command]
pub fn get_document(db: State<Db>, id: i64) -> Result<Doc, String> {
    let conn = db.0.lock().map_err(err)?;
    fetch(&conn, id)
}

#[tauri::command]
pub fn create_document(
    db: State<Db>,
    case_id: i64,
    kind: String,
    title: Option<String>,
) -> Result<Doc, String> {
    if !KINDS.contains(&kind.as_str()) {
        return Err(format!("invalid document kind: {kind}"));
    }
    let conn = db.0.lock().map_err(err)?;
    conn.query_row("SELECT 1 FROM cases WHERE id = ?1", [case_id], |_| Ok(()))
        .map_err(|_| "case file not found".to_string())?;
    conn.execute(
        "INSERT INTO documents (case_id, kind, title) VALUES (?1, ?2, ?3)",
        params![case_id, kind, title.unwrap_or_default().trim()],
    )
    .map_err(err)?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    fetch(&conn, id)
}

#[tauri::command]
pub fn save_document(db: State<Db>, id: i64, input: DocSave) -> Result<Doc, String> {
    let conn = db.0.lock().map_err(err)?;
    let case_id: i64 = conn
        .query_row("SELECT case_id FROM documents WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| "document not found".to_string())?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    tx.execute(
        "UPDATE documents SET title = ?1, content_json = ?2, content_text = ?3, \
         updated_at = datetime('now') WHERE id = ?4",
        params![input.title.trim(), input.content_json, input.content_text, id],
    )
    .map_err(err)?;
    replace_links(&tx, id, case_id, &input.links).map_err(err)?;
    tx.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    tx.commit().map_err(err)?;
    fetch(&conn, id)
}

#[tauri::command]
pub fn delete_document(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    let case_id: i64 = conn
        .query_row("SELECT case_id FROM documents WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| "document not found".to_string())?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    tx.execute("DELETE FROM documents WHERE id = ?1", [id]).map_err(err)?;
    // Polymorphic targets have no foreign key, so clean up links pointing at this doc.
    tx.execute(
        "DELETE FROM links WHERE dst_type = 'document' AND dst_id = ?1",
        [id],
    )
    .map_err(err)?;
    tx.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    tx.commit().map_err(err)
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Markdown,
    Html,
}

/// Writes an article to a path the user picked in the save dialog.
#[tauri::command]
pub fn export_document(
    db: State<Db>,
    id: i64,
    format: ExportFormat,
    path: String,
) -> Result<(), String> {
    let doc = {
        let conn = db.0.lock().map_err(err)?;
        fetch(&conn, id)?
    };
    let out = match format {
        ExportFormat::Markdown => crate::export::to_markdown(&doc.title, &doc.content_json),
        ExportFormat::Html => crate::export::to_html(&doc.title, &doc.content_json, false),
    };
    std::fs::write(&path, out).map_err(|e| format!("could not write {path}: {e}"))
}

/// Opens a print-ready copy in the default browser, where "Save as PDF" is built in.
#[tauri::command]
pub fn open_print_view(
    app: tauri::AppHandle,
    db: State<Db>,
    vault: State<crate::sources::Vault>,
    id: i64,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let doc = {
        let conn = db.0.lock().map_err(err)?;
        fetch(&conn, id)?
    };
    let dir = vault.0.join("exports");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let file = dir.join(format!("article-{id}.html"));
    std::fs::write(&file, crate::export::to_html(&doc.title, &doc.content_json, true)).map_err(err)?;
    app.opener().open_path(file.to_string_lossy(), None::<&str>).map_err(err)
}

/// Documents that link to the given document.
#[tauri::command]
pub fn list_backlinks(db: State<Db>, id: i64) -> Result<Vec<Backlink>, String> {
    let conn = db.0.lock().map_err(err)?;
    backlinks_to(&conn, "document", id).map_err(err)
}

/// Documents that cite the given source.
#[tauri::command]
pub fn list_source_citations(db: State<Db>, id: i64) -> Result<Vec<Backlink>, String> {
    let conn = db.0.lock().map_err(err)?;
    backlinks_to(&conn, "source", id).map_err(err)
}

fn backlinks_to(conn: &Connection, dst_type: &str, id: i64) -> rusqlite::Result<Vec<Backlink>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.case_id, d.kind, d.title FROM links l \
         JOIN documents d ON d.id = l.src_doc_id \
         WHERE l.dst_type = ?1 AND l.dst_id = ?2 ORDER BY d.updated_at DESC",
    )?;
    let rows = stmt.query_map(params![dst_type, id], |r| {
        Ok(Backlink {
            doc_id: r.get(0)?,
            case_id: r.get(1)?,
            kind: r.get(2)?,
            title: r.get(3)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> Connection {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        conn.execute_batch(
            "INSERT INTO cases (code, title) VALUES ('A', 'Case A'), ('B', 'Case B');
             INSERT INTO documents (id, case_id, title) VALUES (1, 1, 'one'), (2, 1, 'two'), (3, 2, 'other case');
             INSERT INTO sources (id, case_id, kind, title) VALUES (10, 1, 'note', 's'), (11, 2, 'note', 'x');",
        )
        .unwrap();
        conn
    }

    fn link(t: &str, id: i64) -> LinkRef {
        LinkRef { dst_type: t.into(), dst_id: id, label: "l".into() }
    }

    #[test]
    fn keeps_only_valid_same_case_links() {
        let conn = setup();
        let links = [
            link("document", 2),  // ok
            link("source", 10),   // ok
            link("document", 3),  // other case: dropped
            link("source", 11),   // other case: dropped
            link("document", 99), // missing: dropped
            link("document", 1),  // self: dropped
            link("bogus", 2),     // unknown type: dropped
            link("document", 2),  // duplicate: ignored
        ];
        replace_links(&conn, 1, 1, &links).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn resaving_replaces_previous_links_and_backlinks_follow() {
        let conn = setup();
        replace_links(&conn, 1, 1, &[link("document", 2)]).unwrap();
        assert_eq!(backlinks_to(&conn, "document", 2).unwrap().len(), 1);
        replace_links(&conn, 1, 1, &[link("source", 10)]).unwrap();
        assert!(backlinks_to(&conn, "document", 2).unwrap().is_empty());
        assert_eq!(backlinks_to(&conn, "source", 10).unwrap()[0].doc_id, 1);
    }

    #[test]
    fn status_flow_only_for_articles_and_tracks_publish_date() {
        let conn = setup();
        conn.execute("INSERT INTO documents (id, case_id, kind, title) VALUES (20, 1, 'article', 'story')", []).unwrap();
        assert!(set_status(&conn, 1, "ready").unwrap_err().contains("only articles"));
        assert!(set_status(&conn, 20, "bogus").is_err());
        assert!(set_status(&conn, 99, "ready").is_err());
        assert_eq!(fetch(&conn, 20).unwrap().status, "draft");
        let d = set_status(&conn, 20, "published").unwrap();
        assert_eq!(d.status, "published");
        let first = d.published_at.clone().unwrap();
        conn.execute("UPDATE documents SET published_at = '2020-01-01 00:00:00' WHERE id = 20", []).unwrap();
        assert_eq!(set_status(&conn, 20, "published").unwrap().published_at.unwrap(), "2020-01-01 00:00:00");
        assert!(!first.is_empty());
        assert!(set_status(&conn, 20, "in_review").unwrap().published_at.is_none());
        let q = queue(&conn).unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].case_code, "A");
    }

    #[test]
    fn deleting_a_doc_removes_its_links() {
        let conn = setup();
        replace_links(&conn, 1, 1, &[link("document", 2)]).unwrap();
        conn.execute("DELETE FROM documents WHERE id = 1", []).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }
    fn party(conn: &Connection, id: i64, name: &str) {
        conn.execute(
            "INSERT INTO parties (id, kind, name) VALUES (?1, 'person', ?2)",
            params![id, name],
        )
        .unwrap();
    }

    #[test]
    fn replace_links_accepts_party_targets_from_any_case_and_skips_missing_ones() {
        let conn = setup();
        party(&conn, 1, "Ann");
        conn.execute(
            "INSERT INTO entities (case_id, kind, name, party_id) VALUES (2, 'person', 'Ann', 1)",
            [],
        )
        .unwrap();
        replace_links(&conn, 1, 1, &[link("party", 1), link("party", 999)]).unwrap();
        let rows: Vec<(String, i64)> = conn
            .prepare("SELECT dst_type, dst_id FROM links WHERE src_doc_id = 1")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows, vec![("party".to_string(), 1)]);
    }

    #[test]
    fn a_notes_backlinks_ignore_party_links() {
        let conn = setup();
        party(&conn, 1, "Ann");
        replace_links(&conn, 1, 1, &[link("party", 1)]).unwrap();
        conn.execute("UPDATE links SET dst_id = 2 WHERE dst_type = 'party'", []).unwrap();
        assert!(backlinks_to(&conn, "document", 2).unwrap().is_empty());
    }

    fn mention(id: &str, label: &str) -> String {
        format!(r#"{{"type":"mention","attrs":{{"id":"{id}","label":"{label}"}}}}"#)
    }

    fn save_content(conn: &Connection, json: &str) {
        conn.execute("UPDATE documents SET content_json = ?1 WHERE id = 1", [json]).unwrap();
    }

    #[test]
    fn fetch_shows_current_party_names() {
        let conn = setup();
        party(&conn, 1, "Old");
        conn.execute("UPDATE parties SET name = 'Renamed' WHERE id = 1", []).unwrap();
        let json = format!(
            r#"{{"type":"doc","content":[{{"type":"paragraph","content":[{},{},{},{}]}},{{"type":"bulletList","content":[{{"type":"listItem","content":[{{"type":"paragraph","content":[{}]}}]}}]}}]}}"#,
            mention("party:1", "Old"),
            mention("party:999", "Gone"),
            mention("doc:3", "Keep"),
            mention("src:3", "Keep2"),
            mention("party:1", "Old")
        );
        save_content(&conn, &json);
        let out = fetch(&conn, 1).unwrap().content_json;
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let p = &v["content"][0]["content"];
        assert_eq!(p[0]["attrs"]["label"], "Renamed");
        assert_eq!(p[1]["attrs"]["label"], "Gone");
        assert_eq!(p[2]["attrs"]["label"], "Keep");
        assert_eq!(p[3]["attrs"]["label"], "Keep2");
        assert_eq!(v["content"][1]["content"][0]["content"][0]["content"][0]["attrs"]["label"], "Renamed");
        save_content(&conn, "not json");
        assert_eq!(fetch(&conn, 1).unwrap().content_json, "not json");
    }

    #[test]
    fn exports_use_the_current_party_name_and_no_ids() {
        let conn = setup();
        party(&conn, 1, "Old");
        conn.execute("UPDATE parties SET name = 'Renamed' WHERE id = 1", []).unwrap();
        let json = format!(
            r#"{{"type":"doc","content":[{{"type":"paragraph","content":[{}]}}]}}"#,
            mention("party:1", "Old")
        );
        save_content(&conn, &json);
        let content = fetch(&conn, 1).unwrap().content_json;
        let md = crate::export::to_markdown("T", &content);
        let html = crate::export::to_html("T", &content, false);
        for out in [md, html] {
            assert!(out.contains("Renamed"));
            assert!(!out.contains("party:"));
        }
    }
}
