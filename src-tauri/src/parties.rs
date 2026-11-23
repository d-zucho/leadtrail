use crate::db::Db;
use crate::search::to_match;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

pub const KINDS: &[&str] = &["person", "organization", "public_body"];

#[derive(Deserialize)]
pub struct PartyInput {
    pub kind: String,
    pub name: String,
    pub aliases: Option<String>,
    pub description: Option<String>,
    pub shared: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct PartyRow {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub aliases: String,
    pub description: String,
    pub shared: bool,
    pub in_this_case: bool,
    pub other_case_count: Option<i64>,
}

fn validate(input: &PartyInput) -> Result<(String, String, String), String> {
    if !KINDS.contains(&input.kind.as_str()) {
        return Err(format!("invalid kind: {}", input.kind));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err("a name is required".into());
    }
    Ok((
        name.to_string(),
        input.aliases.as_deref().unwrap_or("").trim().to_string(),
        input.description.as_deref().unwrap_or("").trim().to_string(),
    ))
}

pub fn create_party(conn: &Connection, input: &PartyInput) -> Result<i64, String> {
    let (name, aliases, description) = validate(input)?;
    conn.execute(
        "INSERT INTO parties (kind, name, aliases, description, shared) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![input.kind, name, aliases, description, input.shared.unwrap_or(false)],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn update_party(conn: &Connection, id: i64, input: &PartyInput) -> Result<(), String> {
    let (name, aliases, description) = validate(input)?;
    let n = conn
        .execute(
            "UPDATE parties SET kind = ?1, name = ?2, aliases = ?3, description = ?4,
                    shared = COALESCE(?5, shared), updated_at = datetime('now')
             WHERE id = ?6",
            params![input.kind, name, aliases, description, input.shared, id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("record not found".into());
    }
    Ok(())
}

pub fn delete_party(conn: &Connection, id: i64) -> Result<(), String> {
    let linked: i64 = conn
        .query_row("SELECT COUNT(*) FROM entities WHERE party_id = ?1", [id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if linked > 0 {
        return Err("This record is linked to one or more case files. Remove those links first.".into());
    }
    let n = conn
        .execute("DELETE FROM parties WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("record not found".into());
    }
    Ok(())
}

pub fn search_parties(
    conn: &Connection,
    q: &str,
    case_id: Option<i64>,
    limit: i64,
) -> rusqlite::Result<Vec<PartyRow>> {
    const COLS: &str = "p.id, p.kind, p.name, p.aliases, p.description, p.shared,
        EXISTS (SELECT 1 FROM entities e WHERE e.party_id = p.id AND e.case_id = ?1),
        CASE WHEN p.shared = 1 THEN
            (SELECT COUNT(*) FROM entities e WHERE e.party_id = p.id AND (?1 IS NULL OR e.case_id <> ?1))
        END";
    let map = |r: &rusqlite::Row| {
        Ok(PartyRow {
            id: r.get(0)?,
            kind: r.get(1)?,
            name: r.get(2)?,
            aliases: r.get(3)?,
            description: r.get(4)?,
            shared: r.get(5)?,
            in_this_case: r.get(6)?,
            other_case_count: r.get(7)?,
        })
    };
    match to_match(q) {
        Some(m) => {
            let mut stmt = conn.prepare(&format!(
                "SELECT {COLS} FROM search s JOIN parties p ON p.id = s.ref_id
                 WHERE search MATCH ?2 AND s.kind = 'party'
                 ORDER BY bm25(search, 5.0, 1.0) LIMIT ?3"
            ))?;
            let rows = stmt.query_map(params![case_id, m, limit], map)?;
            rows.collect()
        }
        None => {
            let mut stmt = conn.prepare(&format!(
                "SELECT {COLS} FROM parties p ORDER BY p.name COLLATE NOCASE LIMIT ?2"
            ))?;
            let rows = stmt.query_map(params![case_id, limit], map)?;
            rows.collect()
        }
    }
}

/// Tauri commands. They live in their own module so the invoke names
/// (`create_party`, ...) can match the plain logic functions above.
pub mod commands {
    use super::*;

    fn err<E: std::fmt::Display>(e: E) -> String {
        e.to_string()
    }

    #[tauri::command]
    pub fn create_party(db: State<Db>, input: PartyInput) -> Result<i64, String> {
        super::create_party(&*db.0.lock().map_err(err)?, &input)
    }

    #[tauri::command]
    pub fn update_party(db: State<Db>, id: i64, input: PartyInput) -> Result<(), String> {
        super::update_party(&*db.0.lock().map_err(err)?, id, &input)
    }

    #[tauri::command]
    pub fn delete_party(db: State<Db>, id: i64) -> Result<(), String> {
        super::delete_party(&*db.0.lock().map_err(err)?, id)
    }

    #[tauri::command]
    pub fn search_parties(
        db: State<Db>,
        query: String,
        case_id: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<PartyRow>, String> {
        super::search_parties(&*db.0.lock().map_err(err)?, &query, case_id, limit.unwrap_or(8)).map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn conn() -> Connection {
        let c = crate::db::open(Path::new(":memory:")).unwrap();
        c.execute_batch(
            "INSERT INTO cases (id, code, title) VALUES (1, 'FN-1', 'Case A'), (2, 'FN-2', 'Case B');",
        )
        .unwrap();
        c
    }

    fn input(kind: &str, name: &str) -> PartyInput {
        PartyInput { kind: kind.into(), name: name.into(), aliases: None, description: None, shared: None }
    }

    fn full(name: &str, aliases: &str, desc: &str, shared: bool) -> PartyInput {
        PartyInput {
            kind: "organization".into(),
            name: name.into(),
            aliases: Some(aliases.into()),
            description: Some(desc.into()),
            shared: Some(shared),
        }
    }

    fn involve(c: &Connection, party: i64, case: i64) {
        c.execute(
            "INSERT INTO entities (case_id, kind, name, party_id) VALUES (?1, 'organization', 'x', ?2)",
            params![case, party],
        )
        .unwrap();
    }

    #[test]
    fn create_validates_kind_and_name() {
        let c = conn();
        assert!(create_party(&c, &input("person", "   ")).is_err());
        assert!(create_party(&c, &input("company", "Acme")).is_err());
        let id = create_party(&c, &full("  Acme  ", " Acme Inc ", " desc ", false)).unwrap();
        let rows = search_parties(&c, "", None, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].name, "Acme");
        assert_eq!(rows[0].aliases, "Acme Inc");
        assert_eq!(rows[0].description, "desc");
        let id2 = create_party(&c, &input("person", "Bob")).unwrap();
        let bob = search_parties(&c, "bob", None, 10).unwrap();
        assert_eq!(bob[0].id, id2);
        assert!(!bob[0].shared);
    }

    #[test]
    fn same_name_records_are_allowed() {
        let c = conn();
        let mut a = input("person", "John Smith");
        a.description = Some("Mayor".into());
        let mut b = input("person", "John Smith");
        b.description = Some("Accountant".into());
        create_party(&c, &a).unwrap();
        create_party(&c, &b).unwrap();
        assert_eq!(search_parties(&c, "john", None, 10).unwrap().len(), 2);
    }

    #[test]
    fn search_matches_name_alias_and_description_by_prefix() {
        let c = conn();
        let id = create_party(&c, &full("Acme Corp", "Acme Corporation", "Shell company", false)).unwrap();
        for q in ["acm", "corporat", "shell"] {
            let rows = search_parties(&c, q, None, 10).unwrap();
            assert_eq!(rows.len(), 1, "query {q}");
            assert_eq!(rows[0].id, id);
        }
    }

    #[test]
    fn search_with_punctuation_does_not_error() {
        let c = conn();
        create_party(&c, &input("person", "Bob O'Brien")).unwrap();
        for q in ["O'Brien \"Bob\"", "acme-corp", "\""] {
            assert!(search_parties(&c, q, None, 10).is_ok(), "query {q}");
        }
    }

    #[test]
    fn blank_query_lists_everything_by_name_nocase() {
        let c = conn();
        for n in ["bravo", "Alpha", "charlie"] {
            create_party(&c, &input("person", n)).unwrap();
        }
        let names: Vec<String> = search_parties(&c, "  ", None, 10).unwrap().into_iter().map(|r| r.name).collect();
        assert_eq!(names, ["Alpha", "bravo", "charlie"]);
        assert_eq!(search_parties(&c, "", None, 2).unwrap().len(), 2);
    }

    #[test]
    fn case_info_appears_only_for_shared_records() {
        let c = conn();
        let shared = create_party(&c, &full("Shared Co", "", "", true)).unwrap();
        let private = create_party(&c, &full("Private Co", "", "", false)).unwrap();
        for p in [shared, private] {
            involve(&c, p, 1);
            involve(&c, p, 2);
        }
        let rows = search_parties(&c, "", Some(1), 10).unwrap();
        let s = rows.iter().find(|r| r.id == shared).unwrap();
        let p = rows.iter().find(|r| r.id == private).unwrap();
        assert!(s.in_this_case && p.in_this_case);
        assert_eq!(s.other_case_count, Some(1));
        assert_eq!(p.other_case_count, None);
        let all = search_parties(&c, "", None, 10).unwrap();
        assert_eq!(all.iter().find(|r| r.id == shared).unwrap().other_case_count, Some(2));
        assert!(!all[0].in_this_case);
    }

    #[test]
    fn update_party_changes_fields_and_toggles_shared() {
        let c = conn();
        let id = create_party(&c, &full("Old", "", "", false)).unwrap();
        c.execute("UPDATE parties SET updated_at = '2000-01-01 00:00:00' WHERE id = ?1", [id]).unwrap();
        update_party(&c, id, &full("New", "Alias", "Desc", true)).unwrap();
        let r = &search_parties(&c, "new", None, 10).unwrap()[0];
        assert_eq!((r.name.as_str(), r.aliases.as_str(), r.description.as_str(), r.shared), ("New", "Alias", "Desc", true));
        let stamp: String = c.query_row("SELECT updated_at FROM parties WHERE id = ?1", [id], |r| r.get(0)).unwrap();
        assert_ne!(stamp, "2000-01-01 00:00:00");
        update_party(&c, id, &input("organization", "New")).unwrap();
        assert!(search_parties(&c, "new", None, 10).unwrap()[0].shared);
        update_party(&c, id, &full("New", "", "", false)).unwrap();
        assert!(!search_parties(&c, "new", None, 10).unwrap()[0].shared);
        assert_eq!(update_party(&c, 999, &input("person", "X")), Err("record not found".to_string()));
    }

    #[test]
    fn delete_is_blocked_while_linked_with_a_generic_message() {
        let c = conn();
        let id = create_party(&c, &input("organization", "Acme")).unwrap();
        involve(&c, id, 2);
        let e = delete_party(&c, id).unwrap_err();
        assert!(!e.contains("Case B") && !e.contains("FN-2"));
        c.execute("DELETE FROM entities WHERE party_id = ?1", [id]).unwrap();
        delete_party(&c, id).unwrap();
        assert!(search_parties(&c, "", None, 10).unwrap().is_empty());
    }
}
