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
    let e = |e: rusqlite::Error| e.to_string();
    let tx = conn.unchecked_transaction().map_err(e)?;
    let linked: i64 = tx
        .query_row("SELECT COUNT(*) FROM entities WHERE party_id = ?1", [id], |r| r.get(0))
        .map_err(e)?;
    if linked > 0 {
        return Err("This record is linked to one or more case files. Remove those links first.".into());
    }
    let exists: i64 = tx
        .query_row("SELECT COUNT(*) FROM parties WHERE id = ?1", [id], |r| r.get(0))
        .map_err(e)?;
    if exists == 0 {
        return Err("record not found".into());
    }
    crate::mentions::detach_party_chips(&tx, id).map_err(e)?;
    tx.execute("DELETE FROM parties WHERE id = ?1", [id]).map_err(e)?;
    tx.execute("DELETE FROM links WHERE dst_type = 'party' AND dst_id = ?1", [id])
        .map_err(e)?;
    tx.commit().map_err(e)
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

#[derive(Serialize)]
pub struct CaseRef {
    pub case_id: i64,
    pub code: String,
    pub title: String,
    pub connections: i64,
}

#[derive(Serialize)]
pub struct PartyDetail {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub aliases: String,
    pub description: String,
    pub shared: bool,
    pub in_this_case: Option<CaseRef>,
    /// Only `Some` when the record is shared; unshared records never reveal other cases.
    pub cases: Option<Vec<CaseRef>>,
}

pub fn link_party(conn: &Connection, case_id: i64, party_id: i64) -> Result<i64, String> {
    conn.query_row("SELECT 1 FROM cases WHERE id = ?1", [case_id], |_| Ok(()))
        .map_err(|_| "case file not found".to_string())?;
    let n = conn
        .execute(
            "INSERT INTO entities (case_id, kind, name, notes, party_id)
             SELECT ?1, kind, name, '', id FROM parties WHERE id = ?2",
            params![case_id, party_id],
        )
        .map_err(|e| match &e {
            rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => {
                "this case file already has that person or organization".to_string()
            }
            _ => e.to_string(),
        })?;
    if n == 0 {
        return Err("record not found".into());
    }
    let id = conn.last_insert_rowid();
    crate::connections::touch_case(conn, case_id)?;
    Ok(id)
}

pub fn create_and_link(conn: &Connection, case_id: i64, input: &PartyInput) -> Result<i64, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let party_id = create_party(&tx, input)?;
    let id = link_party(&tx, case_id, party_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

pub fn get_party(conn: &Connection, id: i64, case_id: Option<i64>) -> Result<PartyDetail, String> {
    let (kind, name, aliases, description, shared): (String, String, String, String, bool) = conn
        .query_row(
            "SELECT kind, name, aliases, description, shared FROM parties WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|_| "record not found".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.code, c.title,
                    (SELECT COUNT(*) FROM relationships r WHERE r.from_id = e.id OR r.to_id = e.id)
             FROM entities e JOIN cases c ON c.id = e.case_id
             WHERE e.party_id = ?1 ORDER BY c.code",
        )
        .map_err(|e| e.to_string())?;
    let all = stmt
        .query_map([id], |r| {
            Ok(CaseRef { case_id: r.get(0)?, code: r.get(1)?, title: r.get(2)?, connections: r.get(3)? })
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .map_err(|e| e.to_string())?;
    let in_this_case = case_id.and_then(|cid| {
        all.iter().find(|c| c.case_id == cid).map(|c| CaseRef { case_id: c.case_id, code: c.code.clone(), title: c.title.clone(), connections: c.connections })
    });
    Ok(PartyDetail {
        id,
        kind,
        name,
        aliases,
        description,
        shared,
        in_this_case,
        cases: if shared { Some(all) } else { None },
    })
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

    #[tauri::command]
    pub fn link_party(db: State<Db>, case_id: i64, party_id: i64) -> Result<i64, String> {
        super::link_party(&*db.0.lock().map_err(err)?, case_id, party_id)
    }

    #[tauri::command]
    pub fn create_party_and_link(db: State<Db>, case_id: i64, input: PartyInput) -> Result<i64, String> {
        super::create_and_link(&*db.0.lock().map_err(err)?, case_id, &input)
    }

    #[tauri::command]
    pub fn get_party(db: State<Db>, id: i64, case_id: Option<i64>) -> Result<PartyDetail, String> {
        super::get_party(&*db.0.lock().map_err(err)?, id, case_id)
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

    #[test]
    fn deleting_a_record_removes_its_mention_links() {
        let c = conn();
        let id = create_party(&c, &input("person", "Ann")).unwrap();
        let other = create_party(&c, &input("person", "Bob")).unwrap();
        c.execute("INSERT INTO documents (id, case_id, title) VALUES (1, 1, 'n')", []).unwrap();
        c.execute(
            "INSERT INTO links (src_doc_id, dst_type, dst_id, label) VALUES (1, 'party', ?1, 'a'), (1, 'party', ?2, 'b'), (1, 'document', ?1, 'c')",
            params![id, other],
        )
        .unwrap();
        delete_party(&c, id).unwrap();
        let n: i64 = c.query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
        let gone: i64 = c
            .query_row("SELECT COUNT(*) FROM links WHERE dst_type = 'party' AND dst_id = ?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(gone, 0);
    }

    fn case_code(c: &Connection, id: i64) -> String {
        c.query_row("SELECT code FROM cases WHERE id = ?1", [id], |r| r.get(0)).unwrap()
    }

    #[test]
    fn link_party_copies_identity_into_the_involvement() {
        let c = conn();
        let p = create_party(&c, &input("public_body", "Water Dept")).unwrap();
        let eid = link_party(&c, 1, p).unwrap();
        let g = crate::connections::load_graph(&c, 1).unwrap();
        assert_eq!(g.entities.len(), 1);
        let e = &g.entities[0];
        assert_eq!((e.id, e.party_id), (eid, p));
        assert_eq!((e.kind.as_str(), e.name.as_str()), ("public_body", "Water Dept"));
    }

    #[test]
    fn same_party_twice_is_rejected_with_a_readable_message() {
        let c = conn();
        let p = create_party(&c, &input("person", "John Smith")).unwrap();
        let q = create_party(&c, &input("person", "John Smith")).unwrap();
        link_party(&c, 1, p).unwrap();
        let e = link_party(&c, 1, p).unwrap_err();
        assert!(e.contains("already"), "got: {e}");
        link_party(&c, 1, q).unwrap();
    }

    #[test]
    fn link_rejects_missing_case_or_party() {
        let c = conn();
        let p = create_party(&c, &input("person", "Bob")).unwrap();
        assert!(link_party(&c, 99, p).is_err());
        assert!(link_party(&c, 1, 99).is_err());
    }

    #[test]
    fn detail_hides_cases_unless_shared() {
        let c = conn();
        let p = create_party(&c, &full("Acme", "", "", false)).unwrap();
        let e1 = link_party(&c, 1, p).unwrap();
        link_party(&c, 2, p).unwrap();
        let other = create_party(&c, &input("person", "Zed")).unwrap();
        let z = link_party(&c, 1, other).unwrap();
        c.execute(
            "INSERT INTO relationships (case_id, from_id, to_id, label) VALUES (1, ?1, ?2, 'owns')",
            params![e1, z],
        )
        .unwrap();
        let d = get_party(&c, p, Some(1)).unwrap();
        assert!(d.cases.is_none());
        let here = d.in_this_case.as_ref().unwrap();
        assert_eq!((here.case_id, here.connections), (1, 1));
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("Case B") && !json.contains(&case_code(&c, 2)), "leaked: {json}");
        assert!(get_party(&c, p, None).unwrap().in_this_case.is_none());

        update_party(&c, p, &full("Acme", "", "", true)).unwrap();
        c.execute(
            "INSERT INTO relationships (case_id, from_id, to_id, label) VALUES (1, ?1, ?2, 'x')",
            params![e1, z],
        )
        .unwrap();
        let d = get_party(&c, p, Some(1)).unwrap();
        let cases = d.cases.unwrap();
        assert_eq!(cases.len(), 2);
        let a = cases.iter().find(|r| r.case_id == 1).unwrap();
        let b = cases.iter().find(|r| r.case_id == 2).unwrap();
        assert_eq!((a.connections, b.connections), (2, 0));
        assert_eq!(b.title, "Case B");
    }

    #[test]
    fn deleting_a_case_keeps_its_records() {
        let c = conn();
        let p = create_party(&c, &input("person", "Bob")).unwrap();
        link_party(&c, 2, p).unwrap();
        c.execute("DELETE FROM cases WHERE id = 2", []).unwrap();
        assert_eq!(search_parties(&c, "", None, 10).unwrap().len(), 1);
        delete_party(&c, p).unwrap();
    }

    #[test]
    fn create_and_link_is_atomic() {
        let c = conn();
        let n = |c: &Connection| -> i64 { c.query_row("SELECT COUNT(*) FROM parties", [], |r| r.get(0)).unwrap() };
        assert!(create_and_link(&c, 1, &input("person", "  ")).is_err());
        assert_eq!(n(&c), 0);
        assert!(create_and_link(&c, 99, &input("person", "Bob")).is_err());
        assert_eq!(n(&c), 0);
        let eid = create_and_link(&c, 1, &input("person", "Bob")).unwrap();
        assert_eq!(n(&c), 1);
        assert_eq!(crate::connections::load_graph(&c, 1).unwrap().entities[0].id, eid);
    }
}
