use crate::db::Db;
use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::Value;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Mention {
    pub doc_id: i64,
    pub case_id: i64,
    pub case_code: Option<String>,
    pub kind: String,
    pub title: String,
}

fn refresh(conn: &Connection, v: &mut Value) -> bool {
    let mut changed = false;
    if v.get("type").and_then(Value::as_str) == Some("mention") {
        let pid = v
            .pointer("/attrs/id")
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("party:"))
            .and_then(|n| n.parse::<i64>().ok());
        if let Some(pid) = pid {
            let name: Option<String> = conn
                .query_row("SELECT name FROM parties WHERE id = ?1", [pid], |r| r.get(0))
                .ok();
            if let Some(name) = name {
                if v.pointer("/attrs/label").and_then(Value::as_str) != Some(name.as_str()) {
                    v["attrs"]["label"] = Value::String(name);
                    changed = true;
                }
            }
        }
    }
    if let Some(children) = v.get_mut("content").and_then(Value::as_array_mut) {
        for c in children {
            changed |= refresh(conn, c);
        }
    }
    changed
}

/// Rewrites party chip labels to the record's current name. Returns the input
/// unchanged when nothing changed or the JSON is not parseable.
pub fn refresh_party_labels(conn: &Connection, content_json: &str) -> String {
    let Ok(mut v) = serde_json::from_str::<Value>(content_json) else {
        return content_json.to_string();
    };
    if refresh(conn, &mut v) {
        serde_json::to_string(&v).unwrap_or_else(|_| content_json.to_string())
    } else {
        content_json.to_string()
    }
}

pub fn party_mentions(
    conn: &Connection,
    party_id: i64,
    case_id: Option<i64>,
) -> Result<Vec<Mention>, String> {
    let shared: i64 = conn
        .query_row("SELECT shared FROM parties WHERE id = ?1", [party_id], |r| r.get(0))
        .map_err(|_| "record not found".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.case_id, c.code, d.kind, d.title FROM links l \
             JOIN documents d ON d.id = l.src_doc_id \
             JOIN cases c ON c.id = d.case_id \
             WHERE l.dst_type = 'party' AND l.dst_id = ?1 \
               AND (d.case_id = ?2 OR ?3 = 1) \
             ORDER BY d.updated_at DESC, d.id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![party_id, case_id, shared], |r| {
            let doc_case: i64 = r.get(1)?;
            let code: String = r.get(2)?;
            Ok(Mention {
                doc_id: r.get(0)?,
                case_id: doc_case,
                case_code: if Some(doc_case) == case_id { None } else { Some(code) },
                kind: r.get(3)?,
                title: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_party_mentions(
    db: State<Db>,
    party_id: i64,
    case_id: Option<i64>,
) -> Result<Vec<Mention>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    party_mentions(&conn, party_id, case_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(shared: bool) -> Connection {
        let c = crate::db::open(std::path::Path::new(":memory:")).unwrap();
        c.execute_batch(
            "INSERT INTO cases (id, code, title) VALUES (1, 'FN-1', 'Alpha case'), (2, 'FN-2', 'Secret case');
             INSERT INTO documents (id, case_id, title) VALUES (1, 1, 'Note one'), (2, 2, 'Hidden note');
             INSERT INTO links (src_doc_id, dst_type, dst_id, label) VALUES (1, 'party', 1, 'p'), (2, 'party', 1, 'p');",
        )
        .unwrap();
        c.execute(
            "INSERT INTO parties (id, kind, name, shared) VALUES (1, 'person', 'Ann', ?1)",
            [shared as i64],
        )
        .unwrap();
        c
    }

    #[test]
    fn unshared_record_lists_only_the_given_cases_notes() {
        let c = setup(false);
        let r = party_mentions(&c, 1, Some(1)).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].doc_id, 1);
        assert_eq!(r[0].case_code, None);
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("FN-2") && !json.contains("Hidden note"));
    }

    #[test]
    fn shared_record_lists_other_cases_with_codes() {
        let c = setup(true);
        let r = party_mentions(&c, 1, Some(1)).unwrap();
        assert_eq!(r.len(), 2);
        let own = r.iter().find(|m| m.doc_id == 1).unwrap();
        let other = r.iter().find(|m| m.doc_id == 2).unwrap();
        assert_eq!(own.case_code, None);
        assert_eq!(other.case_code, Some("FN-2".to_string()));
    }

    #[test]
    fn no_case_context_and_unshared_returns_nothing() {
        let c = setup(false);
        assert!(party_mentions(&c, 1, None).unwrap().is_empty());
    }

    #[test]
    fn no_case_context_and_shared_lists_everything_with_codes() {
        let c = setup(true);
        let r = party_mentions(&c, 1, None).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|m| m.case_code.is_some()));
    }

    #[test]
    fn unknown_record_is_an_error() {
        let c = setup(true);
        assert_eq!(party_mentions(&c, 999, Some(1)).unwrap_err(), "record not found");
    }
}
