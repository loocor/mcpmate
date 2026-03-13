#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = mcpmate_tauri::run() {
        eprintln!("Failed to launch MCPMate Tauri shell: {err:#}");
        std::process::exit(1);
    }
}
