// diorama interactivo de un templo japones renderizado con raytracing.
//
// cada frame: se lee el input (camara + interacciones), se avanzan las
// animaciones, y si algo cambio se vuelve a trazar la imagen. para que
// se pueda mover con fluidez usamos resolucion progresiva: mientras la
// camara se mueve se renderiza chico, y cuando se queda quieta se hace
// una pasada en alta resolucion.

use diorama_templo::ambiente;
use diorama_templo::camara::Camara;
use diorama_templo::escena::Escena;
use diorama_templo::figuras::Interactivo;
use diorama_templo::generador;
use diorama_templo::interaccion::EstadoDiorama;
use diorama_templo::render::{self, Recursos};
use diorama_templo::skybox::Skybox;
use diorama_templo::sonido::Sonidos;
use diorama_templo::textura::Texturas;
use raylib::prelude::*;
use std::time::Instant;

const VENTANA_ANCHO: i32 = 1120;
const VENTANA_ALTO: i32 = 700;

// resoluciones internas del raytracer (misma proporcion 16:10)
const CALIDADES: [(usize, usize); 3] = [(320, 200), (512, 320), (880, 550)];
const NOMBRES_CALIDAD: [&str; 3] = ["previa", "media", "alta"];

struct Lienzo {
    ancho: usize,
    alto: usize,
    buffer: Vec<u8>,
    textura: Texture2D,
}

// rectangulo de la ventana donde se dibuja la imagen, manteniendo la
// proporcion 16:10 (con franjas negras si la ventana tiene otra forma)
fn area_imagen(ventana: (f32, f32)) -> Rectangle {
    let aspecto = CALIDADES[0].0 as f32 / CALIDADES[0].1 as f32;
    let (w, h) = if ventana.0 / ventana.1 > aspecto {
        (ventana.1 * aspecto, ventana.1)
    } else {
        (ventana.0, ventana.0 / aspecto)
    };
    Rectangle::new((ventana.0 - w) * 0.5, (ventana.1 - h) * 0.5, w, h)
}

fn nombre(i: Interactivo) -> &'static str {
    match i {
        Interactivo::Campana => "Campana",
        Interactivo::Linterna => "Linternas",
        Interactivo::Fuente => "Fuente",
        Interactivo::Puerta => "Puerta del templo",
        Interactivo::Ninguno => "",
    }
}

fn main() {
    diorama_templo::silenciar_logs();
    // si faltan las texturas o el skybox, se generan (tarda unos segundos)
    generador::asegurar_assets(false);

    let recursos = Recursos {
        texturas: Texturas::cargar_todas().expect("no se pudieron cargar las texturas"),
        cielos: Skybox::cargar_todos().expect("no se pudo cargar el skybox"),
    };

    // igual que en los proyectos anteriores: con pantallas escaladas de
    // Windows la ventana usa pixeles reales en vez de estirarse borrosa
    unsafe {
        raylib::ffi::SetConfigFlags(raylib::ffi::ConfigFlags::FLAG_WINDOW_HIGHDPI as u32);
    }
    let (mut rl, thread) = raylib::init()
        .size(VENTANA_ANCHO, VENTANA_ALTO)
        .title("Templo japones - diorama con raytracing")
        .log_level(TraceLogLevel::LOG_WARNING)
        .resizable()
        .build();
    rl.set_target_fps(60);

    // audio: si no hay dispositivo de sonido el diorama igual funciona
    let audio = RaylibAudio::init_audio_device().ok();
    let sonidos = audio.as_ref().and_then(|a| match Sonidos::cargar(a) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("sin sonido: {e}");
            None
        }
    });

    let mut lienzos: Vec<Lienzo> = CALIDADES
        .iter()
        .map(|&(ancho, alto)| {
            let img = Image::gen_image_color(ancho as i32, alto as i32, Color::BLACK);
            let textura = rl.load_texture_from_image(&thread, &img).expect("no se pudo crear la textura");
            textura.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            Lienzo { ancho, alto, buffer: vec![0; ancho * alto * 4], textura }
        })
        .collect();

    let mut estado = EstadoDiorama::default();
    let mut escena = Escena::nueva(&estado);
    let mut camara = Camara::vista_general();

    let mut mostrado = 0usize;
    let mut pendiente_alta = true;
    let mut primer_frame = true;
    let mut ms_render = 0.0f32;
    let mut ayuda = true;
    let mut arrastre_desde: Option<Vector2> = None;
    let mut arrastro = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let camara_antes = camara;
        let ventana = (rl.get_screen_width() as f32, rl.get_screen_height() as f32);

        // ------------------------------------------------------------
        // camara
        // ------------------------------------------------------------
        let giro = 1.4 * dt;
        if rl.is_key_down(KeyboardKey::KEY_LEFT) {
            camara.theta -= giro;
        }
        if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            camara.theta += giro;
        }
        if rl.is_key_down(KeyboardKey::KEY_UP) {
            camara.phi += giro * 0.7;
        }
        if rl.is_key_down(KeyboardKey::KEY_DOWN) {
            camara.phi -= giro * 0.7;
        }

        let paso = 7.0 * dt;
        let (mut adelante, mut derecha) = (0.0, 0.0);
        if rl.is_key_down(KeyboardKey::KEY_W) {
            adelante += paso;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            adelante -= paso;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            derecha += paso;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            derecha -= paso;
        }
        if adelante != 0.0 || derecha != 0.0 {
            camara.desplazar(adelante, derecha);
        }
        if rl.is_key_down(KeyboardKey::KEY_Q) {
            camara.objetivo.y += paso * 0.6;
        }
        if rl.is_key_down(KeyboardKey::KEY_Z) {
            camara.objetivo.y -= paso * 0.6;
        }

        // zoom con la rueda o con +/-
        let rueda = rl.get_mouse_wheel_move();
        if rueda != 0.0 {
            camara.radio *= 1.0 - rueda * 0.1;
        }
        if rl.is_key_down(KeyboardKey::KEY_EQUAL) || rl.is_key_down(KeyboardKey::KEY_KP_ADD) {
            camara.radio *= 1.0 - 1.2 * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_MINUS) || rl.is_key_down(KeyboardKey::KEY_KP_SUBTRACT) {
            camara.radio *= 1.0 + 1.2 * dt;
        }

        // arrastrar con el clic izquierdo rota; un clic sin arrastrar interactua
        let mouse = rl.get_mouse_position();
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            arrastre_desde = Some(mouse);
            arrastro = false;
        }
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some(ini) = arrastre_desde {
                if (mouse.x - ini.x).abs() + (mouse.y - ini.y).abs() > 5.0 {
                    arrastro = true;
                }
            }
            if arrastro {
                let delta = rl.get_mouse_delta();
                camara.theta -= delta.x * 0.006;
                camara.phi += delta.y * 0.005;
            }
        }
        let clic = rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) && !arrastro && arrastre_desde.is_some();
        if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
            arrastre_desde = None;
        }

        // vistas predefinidas
        let presets = [
            (KeyboardKey::KEY_ONE, 1),
            (KeyboardKey::KEY_TWO, 2),
            (KeyboardKey::KEY_THREE, 3),
            (KeyboardKey::KEY_FOUR, 4),
            (KeyboardKey::KEY_FIVE, 5),
        ];
        for (tecla, n) in presets {
            if rl.is_key_pressed(tecla) {
                camara = Camara::preset(n);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            camara = Camara::vista_general();
        }
        // ambiente: estacion, ciclo dia/noche y hora a mano
        let mut cambio_ambiente = false;
        if rl.is_key_pressed(KeyboardKey::KEY_C) {
            estado.cambiar_estacion();
            cambio_ambiente = true;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_N) {
            estado.alternar_ciclo();
        }
        if rl.is_key_down(KeyboardKey::KEY_PERIOD) {
            estado.mover_hora(3.0 * dt);
            cambio_ambiente = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_COMMA) {
            estado.mover_hora(-3.0 * dt);
            cambio_ambiente = true;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            ayuda = !ayuda;
        }

        camara.limitar();
        // la camara no puede meterse dentro de los objetos: si el
        // movimiento la deja adentro de alguno, se cancela
        if camara != camara_antes && escena.estatica.contiene(camara.posicion(), 0.3) && !escena.estatica.contiene(camara_antes.posicion(), 0.3) {
            camara = camara_antes;
        }

        // ------------------------------------------------------------
        // interacciones
        // ------------------------------------------------------------
        let (ra_w, ra_h) = (CALIDADES[2].0 as f32, CALIDADES[2].1 as f32);
        let area = area_imagen(ventana);
        let vista = camara.vista();
        let elegir_en = |x: f32, y: f32| {
            let u = (x - area.x) / area.width;
            let v = (y - area.y) / area.height;
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                return None;
            }
            let rayo = vista.rayo(u * ra_w, v * ra_h, ra_w, ra_h);
            render::elegir(&escena, &recursos, &rayo)
        };
        let bajo_mouse = elegir_en(mouse.x, mouse.y).map(|(i, _)| i).unwrap_or(Interactivo::Ninguno);

        let mut accion = None;
        if rl.is_key_pressed(KeyboardKey::KEY_B) {
            accion = Some(Interactivo::Campana);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_L) {
            accion = Some(Interactivo::Linterna);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F) {
            accion = Some(Interactivo::Fuente);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            accion = Some(Interactivo::Puerta);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_E) {
            // interactua con lo que esta en el centro de la pantalla
            let centro = elegir_en(ventana.0 * 0.5, ventana.1 * 0.5).map(|(i, _)| i).unwrap_or(Interactivo::Ninguno);
            if centro == Interactivo::Ninguno {
                estado_mensaje(&mut estado, "No hay nada con que interactuar en el centro de la pantalla.");
            }
            accion = Some(centro);
        }
        if clic && bajo_mouse != Interactivo::Ninguno {
            accion = Some(bajo_mouse);
        }
        let mut cambio_estado = cambio_ambiente;
        if let Some(a) = accion {
            let cambio = estado.interactuar(a);
            if cambio {
                if let Some(s) = &sonidos {
                    s.reproducir(a, &estado, &camara);
                }
            }
            cambio_estado |= cambio;
        }

        let (animando, suave) = estado.actualizar(dt);
        if let Some(s) = &sonidos {
            s.actualizar(&estado, &camara);
        }
        if animando || cambio_estado {
            escena.actualizar(&estado);
        }

        // ------------------------------------------------------------
        // render progresivo
        // ------------------------------------------------------------
        let camara_movida = camara != camara_antes;
        let nivel = if primer_frame {
            Some(0)
        } else if camara_movida || (animando && !suave) || cambio_estado {
            Some(0)
        } else if animando {
            Some(1)
        } else if pendiente_alta {
            Some(2)
        } else {
            None
        };
        if let Some(n) = nivel {
            let lienzo = &mut lienzos[n];
            let inicio = Instant::now();
            render::renderizar(&escena, &recursos, &camara.vista(), lienzo.ancho, lienzo.alto, &mut lienzo.buffer);
            ms_render = inicio.elapsed().as_secs_f32() * 1000.0;
            let _ = lienzo.textura.update_texture(&lienzo.buffer);
            mostrado = n;
            pendiente_alta = n != 2;
            primer_frame = false;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            rl.take_screenshot(&thread, "captura.png");
            estado_mensaje(&mut estado, "Captura guardada en captura.png");
        }

        // ------------------------------------------------------------
        // dibujo
        // ------------------------------------------------------------
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        let lienzo = &lienzos[mostrado];
        d.draw_texture_pro(
            &lienzo.textura,
            Rectangle::new(0.0, 0.0, lienzo.ancho as f32, lienzo.alto as f32),
            area,
            Vector2::new(0.0, 0.0),
            0.0,
            Color::WHITE,
        );
        dibujar_hud(&mut d, &estado, ventana, ayuda, bajo_mouse, mouse, NOMBRES_CALIDAD[mostrado], ms_render, escena.cantidad_objetos());
    }
}

fn estado_mensaje(estado: &mut EstadoDiorama, texto: &str) {
    estado.mensaje = texto.to_string();
    estado.t_mensaje = 3.0;
}

fn panel(d: &mut RaylibDrawHandle, x: i32, y: i32, w: i32, h: i32) {
    d.draw_rectangle(x, y, w, h, Color::new(12, 8, 20, 170));
    d.draw_rectangle_lines(x, y, w, h, Color::new(200, 150, 90, 160));
}

#[allow(clippy::too_many_arguments)]
fn dibujar_hud(
    d: &mut RaylibDrawHandle,
    estado: &EstadoDiorama,
    ventana: (f32, f32),
    ayuda: bool,
    bajo_mouse: Interactivo,
    mouse: Vector2,
    calidad: &str,
    ms: f32,
    objetos: usize,
) {
    let (w, h) = (ventana.0 as i32, ventana.1 as i32);
    let dorado = Color::new(240, 200, 120, 255);
    let claro = Color::new(235, 230, 225, 255);
    let gris = Color::new(170, 165, 175, 255);

    // estacion y hora
    let a = &estado.ambiente;
    let tiempo = format!(
        "{}  -  {}  ({})",
        a.estacion.nombre(),
        ambiente::formato_hora(a.hora),
        if a.ciclo_activo { "el tiempo avanza" } else { "tiempo en pausa" }
    );
    let ancho_tiempo = d.measure_text(&tiempo, 18) + 20;
    panel(d, 10, 10, ancho_tiempo, 30);
    d.draw_text(&tiempo, 20, 16, 18, Color::new(160, 200, 240, 255));

    // controles
    if ayuda {
        let lineas = [
            "Flechas / arrastrar mouse: rotar camara",
            "Rueda o +/-: zoom      WASD: mover   Q/Z: subir/bajar",
            "Clic sobre un objeto o E (centro): interactuar",
            "B campana  L linternas  F fuente  T puerta",
            "C estacion   N ciclo dia/noche   , . cambiar la hora",
            "1-5 vistas   R reiniciar camara   P captura   H ocultar ayuda",
        ];
        let alto = 12 + 20 * lineas.len() as i32;
        panel(d, 10, h - alto - 10, 540, alto);
        for (k, l) in lineas.iter().enumerate() {
            d.draw_text(l, 20, h - alto - 2 + 20 * k as i32, 16, claro);
        }
    }

    // mensaje temporal
    if estado.t_mensaje > 0.0 {
        let alfa = (estado.t_mensaje.min(1.0) * 255.0) as u8;
        let ancho = d.measure_text(&estado.mensaje, 20) + 30;
        let x = (w - ancho) / 2;
        let y = h - 150;
        d.draw_rectangle(x, y, ancho, 36, Color::new(12, 8, 20, alfa.min(190)));
        d.draw_text(&estado.mensaje, x + 15, y + 8, 20, Color::new(250, 220, 160, alfa));
    }

    // mira central (para la tecla E)
    let (cx, cy) = (w / 2, h / 2);
    let mira = Color::new(255, 255, 255, 110);
    d.draw_line(cx - 8, cy, cx + 8, cy, mira);
    d.draw_line(cx, cy - 8, cx, cy + 8, mira);

    // nombre del objeto bajo el mouse
    if bajo_mouse != Interactivo::Ninguno {
        let texto = format!("clic: {}", nombre(bajo_mouse));
        let x = mouse.x as i32 + 16;
        let y = mouse.y as i32 + 12;
        let ancho = d.measure_text(&texto, 16) + 12;
        d.draw_rectangle(x, y, ancho, 22, Color::new(12, 8, 20, 190));
        d.draw_text(&texto, x + 6, y + 3, 16, dorado);
    }

    let info = format!("{} fps | render {}: {:.0} ms | {} primitivas", d.get_fps(), calidad, ms, objetos);
    let ancho = d.measure_text(&info, 14);
    d.draw_text(&info, w - ancho - 10, h - 20, 14, gris);
}
