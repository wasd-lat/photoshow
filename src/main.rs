//! Ponto de entrada: só a janela. O app em si está em `photoshow::app`.

use freya::prelude::*;
use photoshow::app;

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app::app)
                .with_title("photoshow")
                .with_size(1280., 900.)
                .with_min_size(720., 480.),
        ),
    )
}
