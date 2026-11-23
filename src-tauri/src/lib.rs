mod activity;
mod backup;
mod cases;
mod connections;
mod db;
mod documents;
mod export;
mod leads;
mod parties;
mod questions;
mod search;
mod sources;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            // A restore staged by the previous session is applied before the database opens.
            match backup::apply_pending(&dir) {
                Ok(true) => eprintln!("fieldnotes: restored from backup"),
                Ok(false) => {}
                Err(e) => eprintln!("fieldnotes: restore failed, keeping current data: {e}"),
            }
            let conn = db::open(&dir.join("fieldnotes.db"))?;
            app.manage(db::Db(std::sync::Mutex::new(conn)));
            let vault = dir.join("vault");
            std::fs::create_dir_all(vault.join("evidence"))?;
            // Print views are throwaway copies; don't let them pile up.
            let _ = std::fs::remove_dir_all(vault.join("exports"));
            app.manage(sources::Vault(vault));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            db::app_info,
            cases::list_cases,
            cases::get_case,
            cases::create_case,
            cases::update_case,
            cases::delete_case,
            sources::list_sources,
            sources::create_source,
            sources::import_file,
            sources::update_source,
            sources::delete_source,
            sources::open_source_file,
            leads::list_leads,
            leads::create_lead,
            leads::update_lead,
            leads::promote_lead,
            leads::delete_lead,
            documents::list_documents,
            documents::get_document,
            documents::create_document,
            documents::save_document,
            documents::delete_document,
            documents::list_backlinks,
            documents::list_source_citations,
            questions::list_questions,
            questions::create_question,
            questions::update_question,
            questions::delete_question,
            activity::recent_activity,
            parties::commands::create_party,
            parties::commands::update_party,
            parties::commands::delete_party,
            parties::commands::search_parties,
            connections::get_graph,
            connections::create_entity,
            connections::update_entity,
            connections::move_entity,
            connections::delete_entity,
            connections::create_relationship,
            connections::update_relationship,
            connections::delete_relationship,
            connections::set_evidence,
            connections::remove_evidence,
            documents::set_document_status,
            documents::list_queue,
            documents::export_document,
            documents::open_print_view,
            backup::data_info,
            backup::reveal_data_dir,
            backup::backup_vault,
            backup::restore_backup,
            search::search
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
