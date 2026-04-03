#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bootstrap;

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    let _profiler = dhat::Profiler::new_heap();

    bootstrap::run()
}
