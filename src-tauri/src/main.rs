// Pas de fenêtre console sous Windows en version finale (inutile sur macOS, mais sans risque).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    reprise_lib::run()
}
