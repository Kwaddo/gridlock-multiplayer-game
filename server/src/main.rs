mod maze;
mod player;
mod protocols;

mod server;

fn main() -> std::io::Result<()> {
    server::run()
}
