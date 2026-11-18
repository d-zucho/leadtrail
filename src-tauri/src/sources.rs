use crate::db::Db;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::State;
use tauri_plugin_opener::OpenerExt;

/// Root folder of the vault. Evidence lives under `evidence/<case_id>/`.
pub struct Vault(pub PathBuf);

const KINDS: &[&str] = &["file", "link", "note", "interview", "record"];
const VERIFICATION: &[&str] = &["unverified", "to_review", "verified", "disputed"];

#[derive(Serialize)]
pub struct Source {
    pub id: i64,
    pub case_id: i64,
    pub case_code: String,
    pub case_title: String,
    pub kind: String,
    pub title: String,
    pub url: Option<String>,
    pub notes: String,
    pub file_name: Option<String>,
    pub size: Option<i64>,
    pub hash: Option<String>,
    pub verification_status: String,
    pub verification_note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct SourceInput {
    pub case_id: i64,
    pub kind: String,
    pub title: String,
    pub url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct SourceUpdate {
    pub title: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub verification_status: String,
    pub verification_note: Option<String>,
}

const SELECT: &str = "SELECT s.id, s.case_id, c.code, c.title, s.kind, s.title, s.url, s.notes, \
    s.file_name, s.size, s.hash, s.verification_status, s.verification_note, s.created_at, s.updated_at \
    FROM sources s JOIN cases c ON c.id = s.case_id";

fn map(r: &Row) -> rusqlite::Result<Source> {
    Ok(Source {
        id: r.get(0)?,
        case_id: r.get(1)?,
        case_code: r.get(2)?,
        case_title: r.get(3)?,
        kind: r.get(4)?,
        title: r.get(5)?,
        url: r.get(6)?,
        notes: r.get(7)?,
        file_name: r.get(8)?,
        size: r.get(9)?,
        hash: r.get(10)?,
        verification_status: r.get(11)?,
        verification_note: r.get(12)?,
        created_at: r.get(13)?,
        updated_at: r.get(14)?,
    })
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn fetch(conn: &Connection, id: i64) -> Result<Source, String> {
    conn.query_row(&format!("{SELECT} WHERE s.id = ?1"), [id], map)
        .map_err(err)
}

/// Streams the file once, returning (sha256 hex, byte length).
fn hash_file(path: &Path) -> std::io::Result<(String, u64)> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    Ok((hex::encode(hasher.finalize()), total))
}

/// Keeps only a safe file name: no separators, no leading dots.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':') { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim_start_matches('.').trim().to_string();
    if cleaned.is_empty() { "file".into() } else { cleaned }
}

fn case_exists(conn: &Connection, id: i64) -> Result<(), String> {
    conn.query_row("SELECT 1 FROM cases WHERE id = ?1", [id], |_| Ok(()))
        .map_err(|_| "case file not found".to_string())
}

#[tauri::command]
pub fn list_sources(db: State<Db>, case_id: Option<i64>) -> Result<Vec<Source>, String> {
    let conn = db.0.lock().map_err(err)?;
    let mut stmt = conn
        .prepare(&format!(
            "{SELECT} WHERE (?1 IS NULL OR s.case_id = ?1) ORDER BY s.created_at DESC, s.id DESC"
        ))
        .map_err(err)?;
    let rows = stmt.query_map([case_id], map).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

#[tauri::command]
pub fn create_source(db: State<Db>, input: SourceInput) -> Result<Source, String> {
    if !KINDS.contains(&input.kind.as_str()) || input.kind == "file" {
        return Err(format!("invalid source kind: {}", input.kind));
    }
    let title = input.title.trim();
    if title.is_empty() {
        return Err("title is required".into());
    }
    let conn = db.0.lock().map_err(err)?;
    case_exists(&conn, input.case_id)?;
    conn.execute(
        "INSERT INTO sources (case_id, kind, title, url, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            input.case_id,
            input.kind,
            title,
            input.url.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            input.notes.unwrap_or_default().trim()
        ],
    )
    .map_err(err)?;
    touch_case(&conn, input.case_id)?;
    fetch(&conn, conn.last_insert_rowid())
}

/// Copies a file into the vault, hashes it, and records it as a source.
#[tauri::command]
pub fn import_file(
    db: State<Db>,
    vault: State<Vault>,
    case_id: i64,
    path: String,
    title: Option<String>,
) -> Result<Source, String> {
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err("that is not a file".into());
    }
    let (hash, size) = hash_file(&src).map_err(|e| format!("could not read file: {e}"))?;
    let original = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let file_name = sanitize(&original);

    let conn = db.0.lock().map_err(err)?;
    case_exists(&conn, case_id)?;
    let dup: Option<String> = conn
        .query_row(
            "SELECT title FROM sources WHERE case_id = ?1 AND hash = ?2",
            params![case_id, hash],
            |r| r.get(0),
        )
        .ok();
    if let Some(t) = dup {
        return Err(format!("this file is already in the case as \"{t}\""));
    }

    let rel = format!("evidence/{case_id}/{}-{file_name}", &hash[..12]);
    let dest = vault.0.join(&rel);
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(err)?;
    copy_verified(&src, &dest, &hash)?;

    let title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| original.clone());
    let res = conn.execute(
        "INSERT INTO sources (case_id, kind, title, file_path, file_name, size, hash) \
         VALUES (?1, 'file', ?2, ?3, ?4, ?5, ?6)",
        params![case_id, title, rel, file_name, size as i64, hash],
    );
    if let Err(e) = res {
        let _ = std::fs::remove_file(&dest);
        return Err(err(e));
    }
    touch_case(&conn, case_id)?;
    fetch(&conn, conn.last_insert_rowid())
}

/// Copy to a temp name, re-hash the copy, then rename into place so a partial copy never looks valid.
fn copy_verified(src: &Path, dest: &Path, expected: &str) -> Result<(), String> {
    let tmp = dest.with_extension("part");
    {
        let mut from = File::open(src).map_err(err)?;
        let mut to = File::create(&tmp).map_err(err)?;
        std::io::copy(&mut from, &mut to).map_err(err)?;
        to.flush().map_err(err)?;
    }
    let (copied, _) = hash_file(&tmp).map_err(err)?;
    if copied != expected {
        let _ = std::fs::remove_file(&tmp);
        return Err("file changed while importing; try again".into());
    }
    std::fs::rename(&tmp, dest).map_err(err)
}

fn touch_case(conn: &Connection, case_id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE cases SET updated_at = datetime('now') WHERE id = ?1",
        [case_id],
    )
    .map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn update_source(db: State<Db>, id: i64, input: SourceUpdate) -> Result<Source, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("title is required".into());
    }
    if !VERIFICATION.contains(&input.verification_status.as_str()) {
        return Err(format!("invalid verification status: {}", input.verification_status));
    }
    let conn = db.0.lock().map_err(err)?;
    let n = conn
        .execute(
            "UPDATE sources SET title = ?1, url = ?2, notes = ?3, verification_status = ?4, \
             verification_note = ?5, updated_at = datetime('now') WHERE id = ?6",
            params![
                title,
                input.url.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                input.notes.unwrap_or_default().trim(),
                input.verification_status,
                input.verification_note.unwrap_or_default().trim(),
                id
            ],
        )
        .map_err(err)?;
    if n == 0 {
        return Err("source not found".into());
    }
    let s = fetch(&conn, id)?;
    touch_case(&conn, s.case_id)?;
    Ok(s)
}

#[tauri::command]
pub fn delete_source(db: State<Db>, vault: State<Vault>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    let (case_id, file_path): (i64, Option<String>) = conn
        .query_row(
            "SELECT case_id, file_path FROM sources WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "source not found".to_string())?;
    conn.execute("DELETE FROM sources WHERE id = ?1", [id]).map_err(err)?;
    // Links to sources have no foreign key, so drop citations of this source.
    conn.execute("DELETE FROM links WHERE dst_type = 'source' AND dst_id = ?1", [id])
        .map_err(err)?;
    if let Some(p) = file_path {
        let _ = std::fs::remove_file(vault.0.join(p));
    }
    touch_case(&conn, case_id)
}

/// Opens the stored evidence file with the system's default app.
#[tauri::command]
pub fn open_source_file(
    app: tauri::AppHandle,
    db: State<Db>,
    vault: State<Vault>,
    id: i64,
) -> Result<(), String> {
    let rel: Option<String> = {
        let conn = db.0.lock().map_err(err)?;
        conn.query_row("SELECT file_path FROM sources WHERE id = ?1", [id], |r| r.get(0))
            .map_err(|_| "source not found".to_string())?
    };
    let rel = rel.ok_or("this source has no file")?;
    let full = vault.0.join(rel);
    if !full.is_file() {
        return Err("the stored file is missing from the vault".into());
    }
    app.opener()
        .open_path(full.to_string_lossy(), None::<&str>)
        .map_err(err)
}

/// Removes a case's evidence folder. Best effort; the DB rows are already gone.
pub fn remove_case_files(vault: &Vault, case_id: i64) {
    let _ = std::fs::remove_dir_all(vault.0.join(format!("evidence/{case_id}")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize("..."), "file");
        assert_eq!(sanitize("report v2.pdf"), "report v2.pdf");
    }

    #[test]
    fn hashes_known_content() {
        let p = std::env::temp_dir().join("fieldnotes-hash-test.txt");
        std::fs::write(&p, b"abc").unwrap();
        let (h, n) = hash_file(&p).unwrap();
        std::fs::remove_file(&p).ok();
        assert_eq!(n, 3);
        assert_eq!(h, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
