use crate::db::Db;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct Activity {
    /// "case" | "source" | "document" | "lead"
    pub kind: String,
    pub title: String,
    pub case_id: Option<i64>,
    pub case_code: Option<String>,
    /// Set for documents so the UI can link straight to the editor.
    pub doc_id: Option<i64>,
    pub at: String,
}

/// Most recent events across the workspace, newest first.
pub fn recent(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<Activity>> {
    let mut stmt = conn.prepare(
        "SELECT kind, title, case_id, case_code, doc_id, at FROM (
            SELECT 'case' AS kind, title, id AS case_id, code AS case_code, NULL AS doc_id,
                   created_at AS at FROM cases
            UNION ALL
            SELECT 'source', s.title, s.case_id, c.code, NULL, s.created_at
                FROM sources s JOIN cases c ON c.id = s.case_id
            UNION ALL
            SELECT 'document', CASE WHEN d.title = '' THEN 'Untitled' ELSE d.title END,
                   d.case_id, c.code, d.id, d.updated_at
                FROM documents d JOIN cases c ON c.id = d.case_id
            UNION ALL
            SELECT 'lead', l.title, l.case_id, c.code, NULL, l.created_at
                FROM leads l LEFT JOIN cases c ON c.id = l.case_id
        ) ORDER BY at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        Ok(Activity {
            kind: r.get(0)?,
            title: r.get(1)?,
            case_id: r.get(2)?,
            case_code: r.get(3)?,
            doc_id: r.get(4)?,
            at: r.get(5)?,
        })
    })?;
    rows.collect()
}

#[tauri::command]
pub fn recent_activity(db: State<Db>, limit: Option<i64>) -> Result<Vec<Activity>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    recent(&conn, limit.unwrap_or(8).clamp(1, 50)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn merges_event_types_newest_first_and_respects_limit() {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        conn.execute_batch(
            "INSERT INTO cases (id, code, title, created_at) VALUES (1, 'FN-001', 'Case', '2026-01-01 10:00:00');
             INSERT INTO sources (case_id, kind, title, created_at) VALUES (1, 'note', 'src', '2026-01-02 10:00:00');
             INSERT INTO documents (id, case_id, title, updated_at) VALUES (5, 1, '', '2026-01-03 10:00:00');
             INSERT INTO leads (title, created_at) VALUES ('orphan lead', '2026-01-04 10:00:00');",
        )
        .unwrap();

        let all = recent(&conn, 10).unwrap();
        let kinds: Vec<&str> = all.iter().map(|a| a.kind.as_str()).collect();
        assert_eq!(kinds, ["lead", "document", "source", "case"]);
        assert_eq!(all[0].case_code, None); // a lead with no case still appears
        assert_eq!(all[1].title, "Untitled");
        assert_eq!(all[1].doc_id, Some(5));

        assert_eq!(recent(&conn, 2).unwrap().len(), 2);
    }
}
