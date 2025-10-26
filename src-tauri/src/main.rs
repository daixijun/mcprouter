// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Set up the tokio runtime
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    // Block on the async run function
    runtime.block_on(mcprouter_lib::run())
}
