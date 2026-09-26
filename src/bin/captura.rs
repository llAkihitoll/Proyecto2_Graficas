// renderiza una imagen sin abrir ventana y la guarda en PNG. sirve para
// revisar la escena o sacar imagenes para el informe:
//   cargo run --release --bin captura -- <vista 1-5> <etapa 0-4> <ancho> <alto> <archivo.png>
// "etapa" aplica las interacciones en orden: 1 campana, 2 linternas,
// 3 fuente, 4 puerta (con sus animaciones ya terminadas)

use diorama_templo::camara::Camara;
use diorama_templo::escena::Escena;
use diorama_templo::figuras::Interactivo;
use diorama_templo::generador;
use diorama_templo::interaccion::EstadoDiorama;
use diorama_templo::render::{self, Recursos};
use diorama_templo::skybox::Skybox;
use diorama_templo::textura::Texturas;
use raylib::prelude::*;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num = |i: usize, def: u32| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(def);
    let vista = num(1, 1);
    let etapa = num(2, 0);
    let ancho = num(3, 880) as usize;
    let alto = num(4, 550) as usize;
    let salida = args.get(5).cloned().unwrap_or_else(|| "captura.png".to_string());

    diorama_templo::silenciar_logs();
    generador::asegurar_assets(false);
    let recursos = Recursos {
        texturas: Texturas::cargar_todas().expect("texturas"),
        skybox: Skybox::cargar().expect("skybox"),
    };

    let mut estado = EstadoDiorama::default();
    let pasos = [Interactivo::Campana, Interactivo::Linterna, Interactivo::Fuente, Interactivo::Puerta];
    for p in pasos.iter().take(etapa as usize) {
        estado.interactuar(*p);
    }
    // adelantamos las animaciones (la campana queda en pleno balanceo)
    for _ in 0..60 {
        estado.actualizar(0.1);
    }
    if etapa >= 1 {
        estado.t_campana = 0.8;
    }

    let t0 = Instant::now();
    let escena = Escena::nueva(&estado);
    println!(
        "escena: {} primitivas ({} estaticas, {} dinamicas) en {:.0} ms",
        escena.cantidad_objetos(),
        escena.estatica.objetos.len(),
        escena.dinamica.objetos.len(),
        t0.elapsed().as_secs_f32() * 1000.0
    );

    let camara = Camara::preset(vista);
    let mut buffer = vec![0u8; ancho * alto * 4];
    let t1 = Instant::now();
    render::renderizar(&escena, &recursos, &camara.vista(), ancho, alto, &mut buffer);
    println!("render {}x{}: {:.0} ms", ancho, alto, t1.elapsed().as_secs_f32() * 1000.0);

    let mut img = Image::gen_image_color(ancho as i32, alto as i32, Color::BLACK);
    for y in 0..alto {
        for x in 0..ancho {
            let i = (y * ancho + x) * 4;
            img.draw_pixel(x as i32, y as i32, Color::new(buffer[i], buffer[i + 1], buffer[i + 2], 255));
        }
    }
    img.export_image(&salida);
    println!("guardado {salida}");
}
