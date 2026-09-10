//! `rine-manager`: shell desktop do Rine Manager (Tauri 2).
//!
//! Binário fino: registra os commands (todos em `commands`, todos thin
//! adapters sobre `manager-core`) e abre a janela. Lógica aqui = bug
//! arquitetural. Sem GUI, o runtime continua 100% CLI (`rine app.exe`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::detect_runtime,
            commands::inspect_pe,
            commands::preflight,
            commands::run_exe,
            commands::run_registered_app,
            commands::list_apps,
            commands::add_app,
            commands::remove_app,
            commands::set_app_capsule,
            commands::list_capsules,
            commands::create_capsule,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("rine-manager: falha ao abrir a janela principal");
}
