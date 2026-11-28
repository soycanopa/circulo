// Prohíbe console en release en Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    circulo_lib::run();
}