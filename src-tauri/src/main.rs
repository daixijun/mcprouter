// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Set up the tokio runtime with increased stack size to prevent stack overflow
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("mcprouter-runtime")
        .thread_stack_size(8 * 1024 * 1024) // 8MB stack per thread (increased from 4MB to prevent stack overflow)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Block on the async run function
    runtime.block_on(mcprouter_lib::run())
}
