#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod client;
mod i18n;
mod protocol;
mod server;
mod theme;
mod timefmt;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--server" || a == "--server-only") {
        // Порт можно переопределить — удобно для тестов и второго сервера.
        let port = std::env::var("ISKRA_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(protocol::DEFAULT_PORT);
        server::run_headless(port);
        return;
    }

    // Пытаемся поднять локальный сервер. Если порт занят — считаем,
    // что сервер уже запущен (например, другим экземпляром «Искры»).
    let hosting = server::try_start(protocol::DEFAULT_PORT).is_some();
    app::run(hosting);
}
