//! Ponto de entrada: a janela e o `argv` (abrir com o photoshow).
//!
//! Os caminhos do `argv` são lidos **antes** do `launch` e injetados no
//! primeiro frame do app: é o que faz `photoshow ~/Imagens` e o "Abrir com" do
//! gerenciador de arquivos (`Exec=photoshow %F`) abrirem algo.

use std::path::PathBuf;

use freya::prelude::*;
use photoshow::app;

fn main() {
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    app::set_startup_paths(paths);

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app::app)
                .with_title("photoshow")
                .with_size(1280., 900.)
                .with_min_size(720., 480.),
        ),
    )
}
