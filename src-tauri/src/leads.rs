use crate::db::Db;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use tauri::State;

const KINDS: &[&str] = &["note", "link", "tip", "document"];
const STATUSES: &[&str] = &["new", "to_review", "promoted", "dismissed"];

#[derive(Serialize)]
pub struct Lead {
    pub id: i64,
    pub title: String,
    pub kind: String,
    pub url: Option<String>,
    pub body: String,
    pub case_id: Option<i64>,
    pub case_code: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct LeadInput {
    pub title: String,
    pub kind: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    pub case_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct LeadUpdate {
    pub title: String,
    pub url: Option<String>,
    pub body: Option<String>,
    pub case_id: Option<i64>,
    pub status: String,
}

const SELECT: &str = "SELECT l.id, l.title, l.kind, l.url, l.body, l.case_id, c.code, l.status, \
    l.created_at, l.updated_at FROM leads l LEFT JOIN cases c ON c.id = l.case_id";

fn map(r: &Row) -> rusqlite::Result<Lead> {
    Ok(Lead {
        id: r.get(0)?,
        title: r.get(1)?,
        kind: r.get(2)?,
        url: r.get(3)?,
        body: r.get(4)?,
        case_id: r.get(5)?,
        case_code: r.get(6)?,
        status: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn fetch(conn: &Connection, id: i64) -> Result<Lead, String> {
    conn.query_row(&format!("{SELECT} WHERE l.id = ?1"), [id], map)
        .map_err(|_| "lead not found".to_string())
}

fn clean(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

#[tauri::command]
pub fn list_leads(db: State<Db>) -> Result<Vec<Lead>, String> {
    let conn = db.0.lock().map_err(err)?;
    let mut stmt = conn
        .prepare(&format!("{SELECT} ORDER BY l.created_at DESC, l.id DESC"))
        .map_err(err)?;
    let rows = stmt.query_map([], map).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

#[tauri::command]
pub fn create_lead(db: State<Db>, input: LeadInput) -> Result<Lead, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("a lead needs some text".into());
    }
    let kind = input.kind.as_deref().unwrap_or("note");
    if !KINDS.contains(&kind) {
        return Err(format!("invalid lead kind: {kind}"));
    }
    let conn = db.0.lock().map_err(err)?;
    conn.execute(
        "INSERT INTO leads (title, kind, url, body, case_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            title,
            kind,
            clean(input.url.as_deref()),
            input.body.unwrap_or_default().trim(),
            input.case_id
        ],
    )
    .map_err(err)?;
    fetch(&conn, conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_lead(db: State<Db>, id: i64, input: LeadUpdate) -> Result<Lead, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("title is required".into());
    }
    if !STATUSES.contains(&input.status.as_str()) {
        return Err(format!("invalid lead status: {}", input.status));
    }
    let conn = db.0.lock().map_err(err)?;
    let n = conn
        .execute(
            "UPDATE leads SET title = ?1, url = ?2, body = ?3, case_id = ?4, status = ?5, \
             updated_at = datetime('now') WHERE id = ?6",
            params![
                title,
                clean(input.url.as_deref()),
                input.body.unwrap_or_default().trim(),
                input.case_id,
                input.status,
                id
            ],
        )
        .map_err(err)?;
    if n == 0 {
        return Err("lead not found".into());
    }
    fetch(&conn, id)
}

/// Turns a lead into a source in the given case, and marks the lead as promoted.
#[tauri::command]
pub fn promote_lead(db: State<Db>, id: i64, case_id: i64) -> Result<Lead, String> {
    let conn = db.0.lock().map_err(err)?;
    let lead = fetch(&conn, id)?;
    if lead.status == "promoted" {
        return Err("this lead is already in a case file".into());
    }
    let tx = conn.unchecked_transaction().map_err(err)?;
    let exists: bool = tx
        .query_row("SELECT 1 FROM cases WHERE id = ?1", [case_id], |_| Ok(()))
        .is_ok();
    if !exists {
        return Err("case file not found".into());
    }
    let kind = if lead.url.is_some() { "link" } else { "note" };
    tx.execute(
        "INSERT INTO sources (case_id, kind, title, url, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![case_id, kind, lead.title, lead.url, lead.body],
    )
    .map_err(err)?;
    tx.execute(
        "UPDATE leads SET status = 'promoted', case_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![case_id, id],
    )
    .map_err(err)?;
    tx.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    tx.commit().map_err(err)?;
    fetch(&conn, id)
}

#[tauri::command]
pub fn delete_lead(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    conn.execute("DELETE FROM leads WHERE id = ?1", [id]).map_err(err)?;
    Ok(())
}
