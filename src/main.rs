//! Minecraft Dungeons — remake non officiel (fan project, usage privé).
//! Rust + WGPU, dungeon crawler isométrique, co-op locale 2 joueurs.

mod app;
mod assets;
mod consts;
mod gfx;
mod input;
mod models;
mod ui;
mod ui_mcd;
mod world;
mod game;

/// Minimal stderr logger so wgpu/winit diagnostics are visible without env_logger.
struct StderrLogger;

static LEVELS: [(&str, log::Level); 2] = [
    ("wgpu_core", log::Level::Warn),
    ("naga", log::Level::Warn),
];

impl log::Log for StderrLogger {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        if meta.level() > log::Level::Info {
            return false;
        }
        true
    }
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let filter_out = LEVELS
            .iter()
            .any(|(t, l)| record.target().starts_with(t) && record.level() < *l);
        if filter_out {
            return;
        }
        eprintln!("[{} {}] {}", record.level(), record.target(), record.args());
    }
    fn flush(&self) {}
}

fn main() {
    let _ = log::set_logger(&StderrLogger);
    log::set_max_level(log::LevelFilter::Info);
    app::run();
}
