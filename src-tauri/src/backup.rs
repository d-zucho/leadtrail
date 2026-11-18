//! Whole-vault backup and restore.
//!
//! A backup is a single zip: a consistent snapshot of the database plus every evidence
//! file. Restoring never touches the live database. The archive is validated and unpacked
//! into a staging folder, and the swap happens at the next launch, before the database is
//! opened. The data it replaces is kept in a `pre-restore-*` folder.

use rusqlite::Connection;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{Manager, State};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::db::Db;
use crate::sources::Vault;

const DB_ENTRY: &str = "fieldnotes.db";
const MANIFEST: &str = "manifest.json";
const EVIDENCE_PREFIX: &str = "vault/evidence/";
const STAGING: &str = "restore-pending";

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            collect_files(&entry.path(), out)?;
        } else if ft.is_file() {
            out.push(entry.path());
        }
    }
    Ok(())
}

#[derive(Serialize)]
pub struct BackupSummary {
    pub files: usize,
    pub bytes: u64,
}

pub fn create(conn: &Connection, data_dir: &Path, vault: &Path, dest: &Path) -> Result<BackupSummary, String> {
    let snapshot = data_dir.join("backup-snapshot.db");
    let _ = fs::remove_file(&snapshot);
    // VACUUM INTO gives a transactionally consistent copy even while the app is in use.
    conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()]).map_err(err)?;

    let result = write_zip(&snapshot, vault, dest);
    let _ = fs::remove_file(&snapshot);
    if result.is_err() {
        let _ = fs::remove_file(dest);
    }
    result
}

fn write_zip(snapshot: &Path, vault: &Path, dest: &Path) -> Result<BackupSummary, String> {
    let evidence_root = vault.join("evidence");
    let mut files = Vec::new();
    collect_files(&evidence_root, &mut files).map_err(err)?;

    let mut zip = ZipWriter::new(File::create(dest).map_err(|e| format!("could not create {}: {e}", dest.display()))?);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let manifest = serde_json::json!({
        "app": "fieldnotes",
        "schema_version": crate::db::schema_version(),
        "evidence_files": files.len(),
    });
    zip.start_file(MANIFEST, opts).map_err(err)?;
    zip.write_all(manifest.to_string().as_bytes()).map_err(err)?;

    zip.start_file(DB_ENTRY, opts).map_err(err)?;
    let mut bytes = io::copy(&mut File::open(snapshot).map_err(err)?, &mut zip).map_err(err)?;

    for f in &files {
        let rel = f.strip_prefix(&evidence_root).map_err(err)?;
        let name = format!(
            "{EVIDENCE_PREFIX}{}",
            rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
        );
        zip.start_file(name, opts).map_err(err)?;
        bytes += io::copy(&mut File::open(f).map_err(err)?, &mut zip).map_err(err)?;
    }
    zip.finish().map_err(err)?;
    Ok(BackupSummary { files: files.len(), bytes })
}

/// Validates a backup and unpacks it into the staging folder. Nothing live is modified.
pub fn stage_restore(archive: &Path, data_dir: &Path) -> Result<(), String> {
    let staging = data_dir.join(STAGING);
    let _ = fs::remove_dir_all(&staging);
    let result = unpack(archive, &staging);
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn unpack(archive: &Path, staging: &Path) -> Result<(), String> {
    let mut zip = ZipArchive::new(File::open(archive).map_err(err)?).map_err(|_| "this is not a Fieldnotes backup".to_string())?;

    let manifest: serde_json::Value = {
        let mut m = zip.by_name(MANIFEST).map_err(|_| "this is not a Fieldnotes backup".to_string())?;
        serde_json::from_reader(&mut m).map_err(|_| "the backup's manifest is unreadable".to_string())?
    };
    if manifest["app"] != "fieldnotes" {
        return Err("this is not a Fieldnotes backup".into());
    }
    let version = manifest["schema_version"].as_i64().unwrap_or(i64::MAX);
    if version > crate::db::schema_version() {
        return Err("this backup was made by a newer version of Fieldnotes".into());
    }

    fs::create_dir_all(staging.join("vault/evidence")).map_err(err)?;
    let mut saw_db = false;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(err)?;
        if entry.is_dir() {
            continue;
        }
        // `enclosed_name` is None for absolute paths or `..` escapes: refuse the whole archive.
        let name = entry.enclosed_name().ok_or("the backup contains an unsafe path")?;
        let name_str = name.to_string_lossy().replace('\\', "/");
        if name_str == MANIFEST {
            continue;
        }
        let target = if name_str == DB_ENTRY {
            saw_db = true;
            staging.join(DB_ENTRY)
        } else if name_str.starts_with(EVIDENCE_PREFIX) {
            staging.join(&name)
        } else {
            return Err(format!("the backup contains an unexpected file: {name_str}"));
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(err)?;
        }
        io::copy(&mut entry, &mut File::create(&target).map_err(err)?).map_err(err)?;
    }
    if !saw_db {
        return Err("the backup has no database".into());
    }

    let db_path = staging.join(DB_ENTRY);
    let conn = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|_| "the backup's database is damaged".to_string())?;
    if check != "ok" {
        return Err("the backup's database is damaged".into());
    }
    Ok(())
}

/// Called at startup, before the database is opened. Swaps in a staged restore and keeps
/// the previous data in `pre-restore-<timestamp>`. Returns true if a restore was applied.
pub fn apply_pending(data_dir: &Path) -> Result<bool, String> {
    let staging = data_dir.join(STAGING);
    if !staging.join(DB_ENTRY).is_file() {
        let _ = fs::remove_dir_all(&staging);
        return Ok(false);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let old = data_dir.join(format!("pre-restore-{stamp}"));
    fs::create_dir_all(&old).map_err(err)?;

    let live = [DB_ENTRY, "fieldnotes.db-wal", "fieldnotes.db-shm", "vault"];
    let mut moved: Vec<&str> = Vec::new();
    for name in live {
        let from = data_dir.join(name);
        if from.exists() {
            if let Err(e) = fs::rename(&from, old.join(name)) {
                rollback(data_dir, &old, &moved);
                return Err(format!("could not set the current data aside: {e}"));
            }
            moved.push(name);
        }
    }
    let swap = fs::rename(staging.join(DB_ENTRY), data_dir.join(DB_ENTRY)).and_then(|_| {
        let v = staging.join("vault");
        if v.exists() {
            fs::rename(v, data_dir.join("vault"))
        } else {
            Ok(())
        }
    });
    if let Err(e) = swap {
        let _ = fs::remove_file(data_dir.join(DB_ENTRY));
        let _ = fs::remove_dir_all(data_dir.join("vault"));
        rollback(data_dir, &old, &moved);
        return Err(format!("could not apply the restore: {e}"));
    }
    let _ = fs::remove_dir_all(&staging);
    Ok(true)
}

fn rollback(data_dir: &Path, old: &Path, moved: &[&str]) {
    for name in moved {
        let _ = fs::rename(old.join(name), data_dir.join(name));
    }
    let _ = fs::remove_dir(old);
}

#[derive(Serialize)]
pub struct DataInfo {
    pub path: String,
    pub evidence_files: usize,
    pub evidence_bytes: u64,
    pub db_bytes: u64,
}

#[tauri::command]
pub fn data_info(app: tauri::AppHandle, vault: State<Vault>) -> Result<DataInfo, String> {
    let dir = app.path().app_data_dir().map_err(err)?;
    let mut files = Vec::new();
    collect_files(&vault.0.join("evidence"), &mut files).map_err(err)?;
    let evidence_bytes = files.iter().filter_map(|f| fs::metadata(f).ok()).map(|m| m.len()).sum();
    let db_bytes = fs::metadata(dir.join(DB_ENTRY)).map(|m| m.len()).unwrap_or(0);
    Ok(DataInfo {
        path: dir.to_string_lossy().into_owned(),
        evidence_files: files.len(),
        evidence_bytes,
        db_bytes,
    })
}

#[tauri::command]
pub fn reveal_data_dir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = app.path().app_data_dir().map_err(err)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
pub fn backup_vault(
    app: tauri::AppHandle,
    db: State<Db>,
    vault: State<Vault>,
    path: String,
) -> Result<BackupSummary, String> {
    let dir = app.path().app_data_dir().map_err(err)?;
    let conn = db.0.lock().map_err(err)?;
    create(&conn, &dir, &vault.0, Path::new(&path))
}

/// Stages the backup and restarts the app; the swap happens on the next launch.
#[tauri::command]
pub fn restore_backup(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(err)?;
    stage_restore(Path::new(&path), &dir)?;
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fieldnotes-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn seed(dir: &Path, title: &str, evidence: &str) {
        fs::create_dir_all(dir.join("vault/evidence/1")).unwrap();
        fs::write(dir.join("vault/evidence/1/abc-doc.txt"), evidence).unwrap();
        let conn = crate::db::open(&dir.join("fieldnotes.db")).unwrap();
        conn.execute("INSERT INTO cases(code,title) VALUES ('FN-001', ?1)", [title]).unwrap();
    }

    #[test]
    fn backup_then_restore_replaces_everything_and_keeps_the_old_data() {
        let a = scratch("a");
        seed(&a, "Original", "evidence-A");
        let zip_path = a.join("backup.zip");
        let conn = Connection::open(a.join("fieldnotes.db")).unwrap();
        let summary = create(&conn, &a, &a.join("vault"), &zip_path).unwrap();
        assert_eq!(summary.files, 1);
        assert!(!a.join("backup-snapshot.db").exists());
        drop(conn);

        let b = scratch("b");
        seed(&b, "Different", "evidence-B");
        stage_restore(&zip_path, &b).unwrap();
        // staging alone leaves live data untouched
        let live = Connection::open(b.join("fieldnotes.db")).unwrap();
        let t: String = live.query_row("SELECT title FROM cases", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "Different");
        drop(live);

        assert!(apply_pending(&b).unwrap());
        let conn = crate::db::open(&b.join("fieldnotes.db")).unwrap();
        let t: String = conn.query_row("SELECT title FROM cases", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "Original");
        assert_eq!(fs::read_to_string(b.join("vault/evidence/1/abc-doc.txt")).unwrap(), "evidence-A");
        assert!(!b.join(STAGING).exists());

        let old = fs::read_dir(&b)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("pre-restore-"))
            .expect("previous data is kept");
        assert_eq!(fs::read_to_string(old.path().join("vault/evidence/1/abc-doc.txt")).unwrap(), "evidence-B");
        assert!(!apply_pending(&b).unwrap());
    }

    fn zip_with(path: &Path, entries: &[(&str, &[u8])]) {
        let mut z = ZipWriter::new(File::create(path).unwrap());
        for (n, b) in entries {
            z.start_file(*n, SimpleFileOptions::default()).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn rejects_bad_archives() {
        let d = scratch("bad");
        let ok_manifest = format!(r#"{{"app":"fieldnotes","schema_version":{}}}"#, crate::db::schema_version());

        fs::write(d.join("junk.zip"), b"not a zip").unwrap();
        assert!(stage_restore(&d.join("junk.zip"), &d).unwrap_err().contains("not a Fieldnotes"));

        zip_with(&d.join("evil.zip"), &[(MANIFEST, ok_manifest.as_bytes()), ("../evil.txt", b"x")]);
        assert!(stage_restore(&d.join("evil.zip"), &d).unwrap_err().contains("unsafe"));
        assert!(!d.parent().unwrap().join("evil.txt").exists());

        zip_with(&d.join("extra.zip"), &[(MANIFEST, ok_manifest.as_bytes()), ("config.json", b"{}")]);
        assert!(stage_restore(&d.join("extra.zip"), &d).unwrap_err().contains("unexpected"));

        zip_with(&d.join("nodb.zip"), &[(MANIFEST, ok_manifest.as_bytes())]);
        assert!(stage_restore(&d.join("nodb.zip"), &d).unwrap_err().contains("no database"));

        let newer = r#"{"app":"fieldnotes","schema_version":9999}"#;
        zip_with(&d.join("new.zip"), &[(MANIFEST, newer.as_bytes())]);
        assert!(stage_restore(&d.join("new.zip"), &d).unwrap_err().contains("newer"));

        zip_with(&d.join("dmg.zip"), &[(MANIFEST, ok_manifest.as_bytes()), (DB_ENTRY, b"garbage garbage garbage")]);
        assert!(stage_restore(&d.join("dmg.zip"), &d).is_err());
        assert!(!d.join(STAGING).exists(), "failed staging is cleaned up");
    }
}
