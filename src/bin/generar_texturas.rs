// vuelve a generar todas las texturas y el skybox, aunque ya existan:
//   cargo run --release --bin generar_texturas

fn main() {
    diorama_templo::silenciar_logs();
    diorama_templo::generador::asegurar_assets(true);
    println!("listo");
}
