use crate::db::Db;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct Question {
    pub id: i64,
    pub case_id: i64,
    pub case_code: String,
    pub text: String,
    pub due: Option<String>,
    pub done: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct QuestionInput {
    pub text: String,
    pub due: Option<String>,
}

#[derive(Deserialize)]
pub struct QuestionUpdate {
    pub text: String,
    pub due: Option<String>,
    pub done: bool,
}

const SELECT: &str = "SELECT q.id, q.case_id, c.code, q.text, q.due, q.done, q.created_at \
    FROM questions q JOIN cases c ON c.id = q.case_id";

fn map(r: &Row) -> rusqlite::Result<Question> {
    Ok(Question {
        id: r.get(0)?,
        case_id: r.get(1)?,
        case_code: r.get(2)?,
        text: r.get(3)?,
        due: r.get(4)?,
        done: r.get::<_, i64>(5)? != 0,
        created_at: r.get(6)?,
    })
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn fetch(conn: &Connection, id: i64) -> Result<Question, String> {
    conn.query_row(&format!("{SELECT} WHERE q.id = ?1"), [id], map)
        .map_err(|_| "question not found".to_string())
}

/// Accepts an empty value (no due date) or a real YYYY-MM-DD calendar date.
fn clean_due(due: Option<&str>) -> Result<Option<String>, String> {
    let Some(d) = due.map(str::trim).filter(|d| !d.is_empty()) else {
        return Ok(None);
    };
    let b = d.as_bytes();
    let shape_ok = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if shape_ok {
        let year: u32 = d[0..4].parse().unwrap_or(0);
        let month: u32 = d[5..7].parse().unwrap_or(0);
        let day: u32 = d[8..10].parse().unwrap_or(0);
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => 0,
        };
        if year >= 1 && day >= 1 && day <= max_day {
            return Ok(Some(d.to_string()));
        }
    }
    Err("due date must be a real date like 2026-11-30".into())
}

fn touch_case(conn: &Connection, case_id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    Ok(())
}

/// Open questions first, soonest due date first (undated last), then newest.
#[tauri::command]
pub fn list_questions(db: State<Db>, case_id: Option<i64>) -> Result<Vec<Question>, String> {
    let conn = db.0.lock().map_err(err)?;
    let mut stmt = conn
        .prepare(&format!(
            "{SELECT} WHERE (?1 IS NULL OR q.case_id = ?1) \
             ORDER BY q.done ASC, q.due IS NULL, q.due ASC, q.id DESC"
        ))
        .map_err(err)?;
    let rows = stmt.query_map([case_id], map).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

#[tauri::command]
pub fn create_question(
    db: State<Db>,
    case_id: i64,
    input: QuestionInput,
) -> Result<Question, String> {
    let text = input.text.trim();
    if text.is_empty() {
        return Err("a question needs some text".into());
    }
    let due = clean_due(input.due.as_deref())?;
    let conn = db.0.lock().map_err(err)?;
    conn.query_row("SELECT 1 FROM cases WHERE id = ?1", [case_id], |_| Ok(()))
        .map_err(|_| "case file not found".to_string())?;
    conn.execute(
        "INSERT INTO questions (case_id, text, due) VALUES (?1, ?2, ?3)",
        params![case_id, text, due],
    )
    .map_err(err)?;
    touch_case(&conn, case_id)?;
    fetch(&conn, conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_question(
    db: State<Db>,
    id: i64,
    input: QuestionUpdate,
) -> Result<Question, String> {
    let text = input.text.trim();
    if text.is_empty() {
        return Err("a question needs some text".into());
    }
    let due = clean_due(input.due.as_deref())?;
    let conn = db.0.lock().map_err(err)?;
    let n = conn
        .execute(
            "UPDATE questions SET text = ?1, due = ?2, done = ?3, updated_at = datetime('now') \
             WHERE id = ?4",
            params![text, due, input.done as i64, id],
        )
        .map_err(err)?;
    if n == 0 {
        return Err("question not found".into());
    }
    let q = fetch(&conn, id)?;
    touch_case(&conn, q.case_id)?;
    Ok(q)
}

#[tauri::command]
pub fn delete_question(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    let case_id: i64 = conn
        .query_row("SELECT case_id FROM questions WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| "question not found".to_string())?;
    conn.execute("DELETE FROM questions WHERE id = ?1", [id]).map_err(err)?;
    touch_case(&conn, case_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_due_dates() {
        assert_eq!(clean_due(None).unwrap(), None);
        assert_eq!(clean_due(Some("  ")).unwrap(), None);
        assert_eq!(clean_due(Some("2026-11-30")).unwrap().as_deref(), Some("2026-11-30"));
        assert!(clean_due(Some("2028-02-29")).is_ok()); // leap year
        assert!(clean_due(Some("2027-02-29")).is_err());
        assert!(clean_due(Some("2026-13-01")).is_err());
        assert!(clean_due(Some("2026-04-31")).is_err());
        assert!(clean_due(Some("30/11/2026")).is_err());
        assert!(clean_due(Some("2026-1-5")).is_err());
    }
}
