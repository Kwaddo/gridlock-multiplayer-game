use crate::bot::manager::BotManager;

mod maze;
mod player;
mod protocols;
mod math;
mod server;
mod physics;
mod bot;

fn main() -> std::io::Result<()> {
    server::run()
}
