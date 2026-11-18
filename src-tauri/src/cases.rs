use crate::db::Db;
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};
use tauri::State;

const STATUSES: &[&str] = &["investigating", "verification", "drafting", "published", "archived"];

#[derive(Serialize)]
pub struct Case {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub source_count: i64,
    pub document_count: i64,
    pub open_questions: i64,
}

#[derive(Deserialize)]
pub struct CaseInput {
    pub title: String,
    pub summary: Option<String>,
    pub status: Option<String>,
}

const COLS: &str = "id, code, title, summary, status, created_at, updated_at, \
    (SELECT COUNT(*) FROM sources WHERE sources.case_id = cases.id), \
    (SELECT COUNT(*) FROM documents WHERE documents.case_id = cases.id), \
    (SELECT COUNT(*) FROM questions WHERE questions.case_id = cases.id AND questions.done = 0)";

fn map(r: &Row) -> rusqlite::Result<Case> {
    Ok(Case {
        id: r.get(0)?,
        code: r.get(1)?,
        title: r.get(2)?,
        summary: r.get(3)?,
        status: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        source_count: r.get(7)?,
        document_count: r.get(8)?,
        open_questions: r.get(9)?,
    })
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn check_status(s: &str) -> Result<(), String> {
    if STATUSES.contains(&s) {
        Ok(())
    } else {
        Err(format!("invalid status: {s}"))
    }
}

#[tauri::command]
pub fn list_cases(db: State<Db>) -> Result<Vec<Case>, String> {
    let conn = db.0.lock().map_err(err)?;
    let mut stmt = conn
        .prepare(&format!("SELECT {COLS} FROM cases ORDER BY updated_at DESC, id DESC"))
        .map_err(err)?;
    let rows = stmt.query_map([], map).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

#[tauri::command]
pub fn get_case(db: State<Db>, id: i64) -> Result<Case, String> {
    let conn = db.0.lock().map_err(err)?;
    conn.query_row(&format!("SELECT {COLS} FROM cases WHERE id = ?1"), [id], map)
        .map_err(err)
}

#[tauri::command]
pub fn create_case(db: State<Db>, input: CaseInput) -> Result<Case, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("title is required".into());
    }
    let status = input.status.as_deref().unwrap_or("investigating");
    check_status(status)?;

    let conn = db.0.lock().map_err(err)?;
    let next: i64 = conn
        .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM cases", [], |r| r.get(0))
        .map_err(err)?;
    let code = format!("FN-{next:03}");
    conn.execute(
        "INSERT INTO cases (code, title, summary, status) VALUES (?1, ?2, ?3, ?4)",
        params![code, title, input.summary.unwrap_or_default().trim(), status],
    )
    .map_err(err)?;
    let id = conn.last_insert_rowid();
    conn.query_row(&format!("SELECT {COLS} FROM cases WHERE id = ?1"), [id], map)
        .map_err(err)
}

#[tauri::command]
pub fn update_case(db: State<Db>, id: i64, input: CaseInput) -> Result<Case, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("title is required".into());
    }
    let conn = db.0.lock().map_err(err)?;
    let status = match input.status {
        Some(s) => {
            check_status(&s)?;
            s
        }
        None => conn
            .query_row("SELECT status FROM cases WHERE id = ?1", [id], |r| r.get(0))
            .map_err(err)?,
    };
    conn.execute(
        "UPDATE cases SET title = ?1, summary = ?2, status = ?3, updated_at = datetime('now') WHERE id = ?4",
        params![title, input.summary.unwrap_or_default().trim(), status, id],
    )
    .map_err(err)?;
    conn.query_row(&format!("SELECT {COLS} FROM cases WHERE id = ?1"), [id], map)
        .map_err(err)
}

#[tauri::command]
pub fn delete_case(
    db: State<Db>,
    vault: State<crate::sources::Vault>,
    id: i64,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    conn.execute("DELETE FROM cases WHERE id = ?1", [id]).map_err(err)?;
    crate::sources::remove_case_files(&vault, id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::sync::Mutex;

    #[test]
    fn migrations_apply_to_fresh_db() {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM cases", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
        let _ = Db(Mutex::new(conn));
    }

    #[test]
    fn rejects_bad_status() {
        assert!(check_status("investigating").is_ok());
        assert!(check_status("nope").is_err());
    }
}
