mod app;

use app::run;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Multiplayer FPS".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    run().await;
}
