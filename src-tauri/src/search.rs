use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::State;

use crate::db::Db;

/// Highlight markers wrapped around matches. They are control characters, so
/// the frontend can split on them without any HTML ever being injected.
const OPEN: &str = "\u{1}";
const CLOSE: &str = "\u{2}";

#[derive(Serialize, Debug)]
pub struct Hit {
    pub kind: String,
    pub ref_id: i64,
    pub case_id: Option<i64>,
    pub case_code: Option<String>,
    pub title: String,
    pub snippet: String,
}

/// Turns free text into a safe FTS5 query: every word becomes a quoted prefix
/// term, all of which must match. Returns None when nothing searchable remains.
pub(crate) fn to_match(q: &str) -> Option<String> {
    let terms: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

pub fn run(conn: &Connection, q: &str, limit: i64) -> rusqlite::Result<Vec<Hit>> {
    let Some(m) = to_match(q) else {
        return Ok(vec![]);
    };
    let mut stmt = conn.prepare(
        "SELECT s.kind, s.ref_id, s.case_id, c.code, s.title,
                snippet(search, 1, ?2, ?3, '…', 12)
         FROM search s
         LEFT JOIN cases c ON c.id = s.case_id
         WHERE search MATCH ?1
         ORDER BY bm25(search, 5.0, 1.0)
         LIMIT ?4",
    )?;
    let rows = stmt.query_map(params![m, OPEN, CLOSE, limit], |r| {
        Ok(Hit {
            kind: r.get(0)?,
            ref_id: r.get(1)?,
            case_id: r.get(2)?,
            case_code: r.get(3)?,
            title: r.get(4)?,
            snippet: r.get(5)?,
        })
    })?;
    rows.collect()
}

#[tauri::command]
pub fn search(db: State<Db>, query: String) -> Result<Vec<Hit>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    run(&conn, &query, 30).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn db() -> Connection {
        crate::db::open(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn query_is_sanitised() {
        assert_eq!(to_match("  "), None);
        assert_eq!(to_match("\"; DROP -*"), Some("\"DROP\"*".into()));
        assert_eq!(to_match("vance rail"), Some("\"vance\"* \"rail\"*".into()));
    }

    #[test]
    fn finds_and_stays_in_sync() {
        let c = db();
        c.execute("INSERT INTO cases(code,title) VALUES ('FN-001','Harbor contracts')", []).unwrap();
        c.execute(
            "INSERT INTO documents(case_id,title,content_text) VALUES (1,'Draft','The port authority awarded Meridian a contract')",
            [],
        ).unwrap();
        c.execute("INSERT INTO parties(id,kind,name) VALUES (1,'person','Dana Okafor')", []).unwrap();
        c.execute("INSERT INTO entities(case_id,kind,name,party_id) VALUES (1,'person','Dana Okafor',1)", []).unwrap();

        let hits = run(&c, "meridi", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "document");
        assert!(hits[0].snippet.contains("\u{1}Meridian\u{2}"));
        assert_eq!(hits[0].case_code.as_deref(), Some("FN-001"));

        // title-only match across kinds, accents folded
        let kinds: Vec<String> = run(&c, "okafor", 10).unwrap().into_iter().map(|h| h.kind).collect();
        assert!(kinds.contains(&"entity".to_string()) && kinds.contains(&"party".to_string()));
        assert_eq!(run(&c, "harbor", 10).unwrap()[0].kind, "case");

        // update replaces the old text
        c.execute("UPDATE documents SET content_text='Something else' WHERE id=1", []).unwrap();
        assert!(run(&c, "meridian", 10).unwrap().is_empty());
        assert_eq!(run(&c, "something", 10).unwrap().len(), 1);

        // deleting the case cascades out of the index
        c.execute("DELETE FROM cases WHERE id=1", []).unwrap();
        assert!(run(&c, "something", 10).unwrap().is_empty());
        // the entity goes with the case; the global party stays
        let hits = run(&c, "okafor", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "party");
    }

    #[test]
    fn lead_without_case_is_searchable() {
        let c = db();
        c.execute("INSERT INTO leads(title,body) VALUES ('Tip about landfill','anonymous caller')", []).unwrap();
        let h = run(&c, "landfill", 10).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].case_id, None);
    }
}
