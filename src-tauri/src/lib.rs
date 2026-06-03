mod app_config;
mod bookshelf;
pub mod booksource;
mod commands;
mod errors;
mod extensions;
mod fs_utils;
pub mod http;
mod storage;

use app_config::AppConfigState;
use bookshelf::BookshelfState;
use storage::StorageState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(StorageState::new())
        .manage(AppConfigState::new())
        .manage(booksource::BookSourceState::new())
        .manage(BookshelfState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_platform,
            commands::get_local_ips,
            commands::open_dir_in_explorer,
            commands::list_system_fonts,
            commands::cover_cache_size,
            commands::cover_cache_clear,
            commands::cover_resolve_cache,
            commands::booksource_http_proxy,
            commands::web_server_status,
            commands::web_server_start,
            commands::web_server_stop,
            commands::web_server_pick_dist_dir,
            commands::frontend_log,
            commands::not_implemented_command,
            app_config::app_config_get_all,
            app_config::app_config_set,
            app_config::app_config_reset,
            storage::frontend_storage_list,
            storage::frontend_storage_set,
            storage::frontend_storage_remove,
            storage::frontend_storage_list_namespaces,
            storage::storage_debug_dump,
            storage::config_read,
            storage::config_write,
            storage::config_read_json,
            storage::config_write_json,
            storage::config_delete_key,
            storage::config_clear,
            storage::config_read_all,
            storage::config_read_bytes,
            storage::config_write_bytes,
            bookshelf::bookshelf_list,
            bookshelf::bookshelf_add,
            bookshelf::bookshelf_remove,
            bookshelf::bookshelf_get,
            bookshelf::bookshelf_update_book,
            bookshelf::bookshelf_update_progress,
            bookshelf::bookshelf_set_private,
            bookshelf::bookshelf_save_chapters,
            bookshelf::bookshelf_get_chapters,
            bookshelf::bookshelf_save_content,
            bookshelf::bookshelf_get_content,
            bookshelf::bookshelf_delete_content,
            bookshelf::bookshelf_get_cached_indices,
            bookshelf::bookshelf_save_txt_chapters,
            bookshelf::bookshelf_get_episode_progress,
            bookshelf::bookshelf_save_episode_progress,
            bookshelf::bookshelf_restore_source_switch,
            booksource::commands::booksource_get_dir,
            booksource::commands::booksource_get_dirs,
            booksource::commands::booksource_add_dir,
            booksource::commands::booksource_remove_dir,
            booksource::commands::booksource_pick_dir,
            booksource::commands::booksource_list,
            booksource::commands::booksource_list_streaming,
            booksource::commands::booksource_read,
            booksource::commands::booksource_save,
            booksource::commands::booksource_delete,
            booksource::commands::booksource_delete_batch,
            booksource::commands::booksource_toggle,
            booksource::commands::booksource_resolve_path,
            booksource::commands::booksource_open_in_vscode,
            booksource::commands::booksource_import_legacy_json_text,
            booksource::commands::booksource_import_legacy_json_url,
            booksource::commands::booksource_search,
            booksource::commands::booksource_book_info,
            booksource::commands::booksource_chapter_list,
            booksource::commands::booksource_chapter_content,
            booksource::commands::booksource_purchase_chapter,
            booksource::commands::booksource_call_fn,
            booksource::commands::booksource_explore,
            booksource::commands::explore_clear_cache,
            booksource::commands::booksource_cancel,
            booksource::commands::booksource_eval,
            booksource::commands::js_eval,
            booksource::commands::script_repl_eval,
            booksource::commands::booksource_run_tests,
            booksource::commands::booksource_check_update,
            booksource::commands::booksource_apply_update,
            booksource::commands::repository_fetch,
            booksource::commands::repository_install,
            booksource::commands::repository_preview_source,
            booksource::commands::repository_check_source_sync,
            booksource::commands::booksource_save_draft,
            booksource::commands::booksource_delete_draft,
            extensions::extension_get_dir,
            extensions::extension_list,
            extensions::extension_read,
            extensions::extension_save,
            extensions::extension_delete,
            extensions::extension_toggle,
            extensions::extension_open_in_vscode,
        ])
        .setup(|app| {
            let handle = app.handle();
            let storage = handle.state::<StorageState>();
            storage
                .ensure_layout()
                .map_err(|err| format!("初始化数据目录失败: {err}"))?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
