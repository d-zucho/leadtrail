use crate::db::Db;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Serialize)]
pub struct Entity {
    pub id: i64,
    pub case_id: i64,
    pub party_id: i64,
    pub kind: String,
    pub name: String,
    pub notes: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Serialize)]
pub struct Evidence {
    pub source_id: i64,
    pub source_title: String,
    pub source_kind: String,
    pub verification_status: String,
    pub quote: String,
    pub locator: String,
}

#[derive(Serialize)]
pub struct Relationship {
    pub id: i64,
    pub case_id: i64,
    pub from_id: i64,
    pub to_id: i64,
    pub label: String,
    pub notes: String,
    pub evidence: Vec<Evidence>,
}

#[derive(Serialize)]
pub struct Graph {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
}

#[derive(Deserialize)]
pub struct RelationshipInput {
    pub from_id: i64,
    pub to_id: i64,
    pub label: String,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct RelationshipUpdate {
    pub label: String,
    pub notes: Option<String>,
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

/// Turns a unique-constraint failure into a message people can act on.
fn friendly(e: rusqlite::Error, duplicate_msg: &str) -> String {
    match &e {
        rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => {
            duplicate_msg.to_string()
        }
        _ => e.to_string(),
    }
}

fn exists_in_case(conn: &Connection, table: &str, id: i64, case_id: i64) -> bool {
    conn.query_row(
        &format!("SELECT 1 FROM {table} WHERE id = ?1 AND case_id = ?2"),
        params![id, case_id],
        |_| Ok(()),
    )
    .is_ok()
}

pub fn load_graph(conn: &Connection, case_id: i64) -> rusqlite::Result<Graph> {
    let mut stmt = conn.prepare(
        "SELECT id, case_id, kind, name, notes, x, y, party_id FROM entities WHERE case_id = ?1 ORDER BY id",
    )?;
    let entities = stmt
        .query_map([case_id], |r| {
            Ok(Entity {
                id: r.get(0)?,
                case_id: r.get(1)?,
                kind: r.get(2)?,
                name: r.get(3)?,
                notes: r.get(4)?,
                x: r.get(5)?,
                y: r.get(6)?,
                party_id: r.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // All evidence for the case in one query, grouped by relationship below.
    let mut ev_stmt = conn.prepare(
        "SELECT rs.relationship_id, s.id, s.title, s.kind, s.verification_status, rs.quote, rs.locator \
         FROM relationship_sources rs \
         JOIN relationships r ON r.id = rs.relationship_id \
         JOIN sources s ON s.id = rs.source_id \
         WHERE r.case_id = ?1 ORDER BY rs.created_at, s.id",
    )?;
    let mut evidence: HashMap<i64, Vec<Evidence>> = HashMap::new();
    let rows = ev_stmt.query_map([case_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            Evidence {
                source_id: r.get(1)?,
                source_title: r.get(2)?,
                source_kind: r.get(3)?,
                verification_status: r.get(4)?,
                quote: r.get(5)?,
                locator: r.get(6)?,
            },
        ))
    })?;
    for row in rows {
        let (rid, e) = row?;
        evidence.entry(rid).or_default().push(e);
    }

    let mut rel_stmt = conn.prepare(
        "SELECT id, case_id, from_id, to_id, label, notes FROM relationships \
         WHERE case_id = ?1 ORDER BY id",
    )?;
    let relationships = rel_stmt
        .query_map([case_id], |r| {
            let id: i64 = r.get(0)?;
            Ok(Relationship {
                id,
                case_id: r.get(1)?,
                from_id: r.get(2)?,
                to_id: r.get(3)?,
                label: r.get(4)?,
                notes: r.get(5)?,
                evidence: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|mut rel| {
            rel.evidence = evidence.remove(&rel.id).unwrap_or_default();
            rel
        })
        .collect();

    Ok(Graph { entities, relationships })
}

pub fn add_relationship(
    conn: &Connection,
    case_id: i64,
    input: &RelationshipInput,
) -> Result<i64, String> {
    let label = input.label.trim();
    if label.is_empty() {
        return Err("describe the relationship, e.g. \"director of\"".into());
    }
    if input.from_id == input.to_id {
        return Err("an entity cannot be connected to itself".into());
    }
    if !exists_in_case(conn, "entities", input.from_id, case_id)
        || !exists_in_case(conn, "entities", input.to_id, case_id)
    {
        return Err("both entities must belong to this case file".into());
    }
    conn.execute(
        "INSERT INTO relationships (case_id, from_id, to_id, label, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            case_id,
            input.from_id,
            input.to_id,
            label,
            input.notes.as_deref().unwrap_or("").trim()
        ],
    )
    .map_err(|e| friendly(e, "these two already have a relationship with that label"))?;
    Ok(conn.last_insert_rowid())
}

/// Adds or replaces the evidence a source gives for a relationship.
pub fn put_evidence(
    conn: &Connection,
    relationship_id: i64,
    source_id: i64,
    quote: &str,
    locator: &str,
) -> Result<(), String> {
    let case_id: i64 = conn
        .query_row(
            "SELECT case_id FROM relationships WHERE id = ?1",
            [relationship_id],
            |r| r.get(0),
        )
        .map_err(|_| "relationship not found".to_string())?;
    if !exists_in_case(conn, "sources", source_id, case_id) {
        return Err("that source is not in this case file".into());
    }
    conn.execute(
        "INSERT INTO relationship_sources (relationship_id, source_id, quote, locator) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (relationship_id, source_id) DO UPDATE SET quote = ?3, locator = ?4",
        params![relationship_id, source_id, quote.trim(), locator.trim()],
    )
    .map_err(err)?;
    Ok(())
}

pub(crate) fn touch_case(conn: &Connection, case_id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn get_graph(db: State<Db>, case_id: i64) -> Result<Graph, String> {
    let conn = db.0.lock().map_err(err)?;
    load_graph(&conn, case_id).map_err(err)
}

/// Notes are the only per-case field on an involvement; name and kind belong to the record.
pub fn set_notes(conn: &Connection, id: i64, notes: &str) -> Result<(), String> {
    let n = conn
        .execute("UPDATE entities SET notes = ?1 WHERE id = ?2", params![notes.trim(), id])
        .map_err(err)?;
    if n == 0 {
        return Err("entity not found".into());
    }
    Ok(())
}

#[tauri::command]
pub fn update_entity(db: State<Db>, id: i64, notes: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    set_notes(&conn, id, &notes)
}

/// Remembers where a node was dragged to.
#[tauri::command]
pub fn move_entity(db: State<Db>, id: i64, x: f64, y: f64) -> Result<(), String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("invalid position".into());
    }
    let conn = db.0.lock().map_err(err)?;
    conn.execute("UPDATE entities SET x = ?1, y = ?2 WHERE id = ?3", params![x, y, id])
        .map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn delete_entity(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    conn.execute("DELETE FROM entities WHERE id = ?1", [id]).map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn create_relationship(
    db: State<Db>,
    case_id: i64,
    input: RelationshipInput,
) -> Result<i64, String> {
    let conn = db.0.lock().map_err(err)?;
    let id = add_relationship(&conn, case_id, &input)?;
    touch_case(&conn, case_id)?;
    Ok(id)
}

#[tauri::command]
pub fn update_relationship(
    db: State<Db>,
    id: i64,
    input: RelationshipUpdate,
) -> Result<(), String> {
    let label = input.label.trim();
    if label.is_empty() {
        return Err("describe the relationship, e.g. \"director of\"".into());
    }
    let conn = db.0.lock().map_err(err)?;
    let n = conn
        .execute(
            "UPDATE relationships SET label = ?1, notes = ?2 WHERE id = ?3",
            params![label, input.notes.unwrap_or_default().trim(), id],
        )
        .map_err(|e| friendly(e, "these two already have a relationship with that label"))?;
    if n == 0 {
        return Err("relationship not found".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_relationship(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    conn.execute("DELETE FROM relationships WHERE id = ?1", [id]).map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn set_evidence(
    db: State<Db>,
    relationship_id: i64,
    source_id: i64,
    quote: Option<String>,
    locator: Option<String>,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    put_evidence(
        &conn,
        relationship_id,
        source_id,
        quote.as_deref().unwrap_or(""),
        locator.as_deref().unwrap_or(""),
    )
}

#[tauri::command]
pub fn remove_evidence(
    db: State<Db>,
    relationship_id: i64,
    source_id: i64,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    conn.execute(
        "DELETE FROM relationship_sources WHERE relationship_id = ?1 AND source_id = ?2",
        params![relationship_id, source_id],
    )
    .map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> Connection {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        conn.execute_batch(
            "INSERT INTO cases (id, code, title) VALUES (1, 'A', 'Case A'), (2, 'B', 'Case B');
             INSERT INTO parties (id, kind, name) VALUES
                (1, 'organization', 'Northbank'), (2, 'public_body', 'Water dept'), (3, 'person', 'Maya');
             INSERT INTO entities (id, case_id, kind, name, party_id) VALUES
                (1, 1, 'organization', 'Northbank', 1), (2, 1, 'public_body', 'Water dept', 2),
                (3, 2, 'person', 'Maya', 3);
             INSERT INTO sources (id, case_id, kind, title, verification_status) VALUES
                (10, 1, 'record', 'Award notice', 'verified'), (11, 2, 'note', 'Elsewhere', 'unverified');",
        )
        .unwrap();
        conn
    }

    fn rel(from: i64, to: i64, label: &str) -> RelationshipInput {
        RelationshipInput { from_id: from, to_id: to, label: label.into(), notes: None }
    }

    #[test]
    fn rejects_self_loops_cross_case_and_blank_labels() {
        let conn = setup();
        assert!(add_relationship(&conn, 1, &rel(1, 1, "x")).is_err());
        assert!(add_relationship(&conn, 1, &rel(1, 3, "x")).is_err()); // entity 3 is in case 2
        assert!(add_relationship(&conn, 1, &rel(1, 2, "  ")).is_err());
        assert!(add_relationship(&conn, 1, &rel(1, 2, "awarded")).is_ok());
    }

    #[test]
    fn duplicate_relationship_gets_a_readable_error_ignoring_case() {
        let conn = setup();
        add_relationship(&conn, 1, &rel(2, 1, "Awarded")).unwrap();
        let e = add_relationship(&conn, 1, &rel(2, 1, "awarded")).unwrap_err();
        assert!(e.contains("already have a relationship"), "got: {e}");
        // A different label, or the reverse direction, is a different fact.
        assert!(add_relationship(&conn, 1, &rel(2, 1, "regulates")).is_ok());
        assert!(add_relationship(&conn, 1, &rel(1, 2, "Awarded")).is_ok());
    }

    #[test]
    fn evidence_must_come_from_the_same_case_and_upserts() {
        let conn = setup();
        let rid = add_relationship(&conn, 1, &rel(2, 1, "awarded")).unwrap();
        assert!(put_evidence(&conn, rid, 11, "q", "p. 1").is_err()); // source in case 2
        put_evidence(&conn, rid, 10, "first", "p. 4").unwrap();
        put_evidence(&conn, rid, 10, "second", "p. 5").unwrap(); // replaces, no duplicate row

        let g = load_graph(&conn, 1).unwrap();
        assert_eq!(g.entities.len(), 2);
        assert_eq!(g.relationships.len(), 1);
        let ev = &g.relationships[0].evidence;
        assert_eq!(ev.len(), 1);
        assert_eq!((ev[0].quote.as_str(), ev[0].locator.as_str()), ("second", "p. 5"));
        assert_eq!(ev[0].verification_status, "verified");
    }

    #[test]
    fn deleting_things_cleans_up_dependents() {
        let conn = setup();
        let rid = add_relationship(&conn, 1, &rel(2, 1, "awarded")).unwrap();
        put_evidence(&conn, rid, 10, "q", "").unwrap();

        // Deleting the source drops its evidence but keeps the relationship.
        conn.execute("DELETE FROM sources WHERE id = 10", []).unwrap();
        let g = load_graph(&conn, 1).unwrap();
        assert_eq!(g.relationships.len(), 1);
        assert!(g.relationships[0].evidence.is_empty());

        // Deleting an entity drops its relationships.
        conn.execute("DELETE FROM entities WHERE id = 1", []).unwrap();
        assert!(load_graph(&conn, 1).unwrap().relationships.is_empty());
    }

    #[test]
    fn same_party_cannot_be_linked_twice_but_same_name_can_repeat() {
        let conn = setup();
        // Party 1 is already linked to case 1.
        assert!(conn
            .execute("INSERT INTO entities (case_id, kind, name, party_id) VALUES (1, 'organization', 'Northbank', 1)", [])
            .is_err());
        // A different party with the same name can be linked to the same case.
        conn.execute("INSERT INTO parties (id, kind, name) VALUES (4, 'organization', 'Northbank')", []).unwrap();
        assert!(conn
            .execute("INSERT INTO entities (case_id, kind, name, party_id) VALUES (1, 'organization', 'Northbank', 4)", [])
            .is_ok());
        // The same party can be linked to another case.
        assert!(conn
            .execute("INSERT INTO entities (case_id, kind, name, party_id) VALUES (2, 'organization', 'Northbank', 1)", [])
            .is_ok());
    }

    #[test]
    fn update_entity_only_changes_notes() {
        let conn = setup();
        set_notes(&conn, 1, "  hello ").unwrap();
        let g = load_graph(&conn, 1).unwrap();
        let e = g.entities.iter().find(|e| e.id == 1).unwrap();
        assert_eq!((e.notes.as_str(), e.name.as_str(), e.kind.as_str()), ("hello", "Northbank", "organization"));
        assert_eq!(e.party_id, 1);
        assert!(set_notes(&conn, 99, "x").is_err());
    }
}
