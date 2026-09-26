// genera las texturas (assets/textures/*.png) y las 6 caras del skybox
// (assets/skybox/*.png) de forma procedural y las guarda como imagenes.
// el diorama despues las CARGA desde archivo como cualquier textura, asi
// que se pueden reemplazar por imagenes propias sin tocar el codigo.
//
// se ejecuta solo si faltan archivos, o a mano con:
//   cargo run --release --bin generar_texturas

use crate::matematica::{suavizar, v3, Vec3};
use crate::ruido::{fbm, hash, hash2, valor2, voronoi};
use crate::skybox::{self, CARAS, CARPETA_SKYBOX};
use crate::textura::{ARCHIVOS, CARPETA_TEXTURAS};
use raylib::prelude::*;
use std::f32::consts::PI;
use std::path::Path;

const LADO_TEXTURA: i32 = 256;
const LADO_CIELO: i32 = 768;
// muestras por lado de cada pixel (supersampling): suaviza bordes como la
// silueta de las montanas, que si no se ven escalonadas al ampliarse
const SUBMUESTRAS: i32 = 2;

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn a_color(c: Vec3) -> Color {
    let f = |x: f32| (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    Color::new(f(c.x), f(c.y), f(c.z), 255)
}

// calcula la imagen repartiendo las filas entre los nucleos y la guarda
fn guardar(ruta: &str, lado: i32, f: &(dyn Fn(f32, f32) -> Vec3 + Sync)) {
    let n = lado as usize;
    let mut colores = vec![Vec3::CERO; n * n];
    let hilos = std::thread::available_parallelism().map_or(4, |h| h.get());
    let filas_por_hilo = n.div_ceil(hilos);
    std::thread::scope(|s| {
        for (bloque, trozo) in colores.chunks_mut(filas_por_hilo * n).enumerate() {
            s.spawn(move || {
                for (k, c) in trozo.iter_mut().enumerate() {
                    let x = k % n;
                    let y = bloque * filas_por_hilo + k / n;
                    let mut suma = Vec3::CERO;
                    for sy in 0..SUBMUESTRAS {
                        for sx in 0..SUBMUESTRAS {
                            let u = (x as f32 + (sx as f32 + 0.5) / SUBMUESTRAS as f32) / lado as f32;
                            let v = (y as f32 + (sy as f32 + 0.5) / SUBMUESTRAS as f32) / lado as f32;
                            suma += f(u, v);
                        }
                    }
                    *c = suma / (SUBMUESTRAS * SUBMUESTRAS) as f32;
                }
            });
        }
    });
    let mut img = Image::gen_image_color(lado, lado, Color::BLACK);
    for (k, c) in colores.iter().enumerate() {
        img.draw_pixel((k % n) as i32, (k / n) as i32, a_color(*c));
    }
    img.export_image(ruta);
}

// ---------------------------------------------------------------------
// texturas de materiales. todas reciben u, v en [0, 1) y son periodicas
// ---------------------------------------------------------------------

// madera: tablones verticales con vetas que ondulan y fibras finas
fn tex_madera(u: f32, v: f32) -> Vec3 {
    let tablon = (u * 4.0).floor();
    let tono = 0.85 + 0.3 * hash2(tablon as i32, 3, 11);
    let onda = fbm(u, v, 4, 4, 1);
    let veta = 0.5 + 0.5 * (2.0 * PI * (u * 16.0 + onda * 2.5)).sin();
    let veta = veta.powf(3.0);
    let fibras = valor2(u * 128.0, v * 6.0, 0, 5);
    let claro = v3(0.64, 0.44, 0.26);
    let oscuro = v3(0.36, 0.22, 0.12);
    let mut c = claro.lerp(oscuro, veta * 0.65 + fibras * 0.35) * tono;
    // junta entre tablones
    let fu = fract(u * 4.0);
    if fu < 0.018 || fu > 0.982 {
        c *= 0.5;
    }
    c
}

// piedra: losas irregulares (voronoi) con juntas oscuras y algo de musgo
fn tex_piedra(u: f32, v: f32) -> Vec3 {
    let (f1, f2, id) = voronoi(u, v, 5, 21);
    let borde = f2 - f1;
    let junta = 1.0 - suavizar(0.03, 0.10, borde);
    let tono = 0.44 + 0.2 * hash(id.wrapping_mul(2_654_435_761));
    let grano = fbm(u, v, 16, 3, 23);
    let mut c = v3(tono, tono * 0.97, tono * 0.92) * (0.8 + 0.4 * grano);
    // la cara de cada piedra se aclara hacia el centro (volumen)
    c *= 0.85 + 0.25 * (1.0 - f1).clamp(0.0, 1.0);
    let musgo = suavizar(0.55, 0.7, fbm(u, v, 4, 4, 29));
    let color_junta = v3(0.17, 0.17, 0.15).lerp(v3(0.22, 0.30, 0.14), musgo);
    c.lerp(color_junta, junta)
}

// metal (bronce viejo): cepillado horizontal con manchas de patina verde
fn tex_metal(u: f32, v: f32) -> Vec3 {
    let cepillado = valor2(u * 4.0, v * 256.0, 4, 31);
    let base = v3(0.60, 0.43, 0.22) * (0.85 + 0.25 * cepillado);
    let patina = suavizar(0.58, 0.72, fbm(u, v, 4, 5, 33));
    base.lerp(v3(0.30, 0.52, 0.44), patina * 0.8)
}

// agua: verde azulado con lineas brillantes tipo caustica
fn tex_agua(u: f32, v: f32) -> Vec3 {
    let n = fbm(u, v, 3, 4, 41);
    let c = 1.0 - (2.0 * PI * n * 3.0).sin().abs();
    let caustica = c.powf(8.0);
    v3(0.10, 0.34, 0.40) + v3(0.25, 0.35, 0.35) * caustica
}

// vidrio: casi blanco con un tinte cian, estrias finas y burbujitas
fn tex_vidrio(u: f32, v: f32) -> Vec3 {
    let estria = valor2(u * 64.0, v * 2.0, 0, 51);
    let (f1, _, _) = voronoi(u, v, 9, 53);
    let burbuja = if f1 < 0.07 && f1 > 0.045 { 0.08 } else { 0.0 };
    v3(0.84, 0.95, 0.97) * (0.95 + 0.05 * estria) + Vec3::UNO * burbuja
}

// tejas japonesas (kawara): columnas redondeadas que bajan por la
// pendiente, con la sombra donde cada fila se monta sobre la de abajo
fn tex_tejas(u: f32, v: f32) -> Vec3 {
    let columnas = 8.0;
    let filas = 6.0;
    let cu = fract(u * columnas);
    let fv = fract(v * filas);
    let col = (u * columnas).floor() as i32;
    let fila = (v * filas).floor() as i32;
    // perfil redondo de la teja (0 en el canal, 1 en la cresta)
    let perfil = (PI * cu).sin().powf(0.6);
    let mut sombra = 0.35 + 0.65 * perfil;
    // superposicion con la fila de arriba
    sombra *= 0.6 + 0.4 * suavizar(0.0, 0.3, fv);
    if fv > 0.96 {
        sombra *= 0.55;
    }
    let tono = 0.85 + 0.3 * hash2(col, fila, 61);
    let grano = 0.9 + 0.1 * fbm(u, v, 8, 3, 63);
    v3(0.25, 0.28, 0.34) * (sombra * tono * grano * 1.25)
}

// papel de shoji / yeso: blanco calido con fibras
fn tex_papel(u: f32, v: f32) -> Vec3 {
    let fibras = valor2(u * 96.0, v * 12.0, 0, 71) * 0.5 + fbm(u, v, 8, 3, 73) * 0.5;
    v3(0.95, 0.92, 0.85) * (0.9 + 0.1 * fibras)
}

// follaje: manchones de hojas. un verde amarillento neutro, el tinte de
// cada material (pino, arce rojo, sakura) le da el color final
fn tex_follaje(u: f32, v: f32) -> Vec3 {
    let (f1, _, id) = voronoi(u, v, 14, 81);
    let hoja = (1.0 - f1 * 1.4).clamp(0.0, 1.0);
    let n = fbm(u, v, 6, 4, 83);
    let oscuro = v3(0.32, 0.36, 0.30);
    let claro = v3(0.86, 0.9, 0.8);
    let t = (n * 0.5 + hoja * 0.5) * (0.8 + 0.4 * hash(id.wrapping_mul(97)));
    oscuro.lerp(claro, t.clamp(0.0, 1.0))
}

// pasto / musgo
fn tex_pasto(u: f32, v: f32) -> Vec3 {
    let n = fbm(u, v, 6, 5, 91);
    let briznas = valor2(u * 128.0, v * 128.0, 128, 93);
    let c = v3(0.26, 0.40, 0.15).lerp(v3(0.44, 0.56, 0.22), n);
    c * (0.8 + 0.35 * briznas)
}

// grava rastrillada (jardin zen): surcos paralelos y piedritas
fn tex_grava(u: f32, v: f32) -> Vec3 {
    let surco = 0.5 + 0.5 * (2.0 * PI * (v * 10.0 + 0.05 * fbm(u, v, 4, 3, 101))).sin();
    let (f1, _, id) = voronoi(u, v, 48, 103);
    let piedrita = (1.0 - f1 * 1.6).clamp(0.0, 1.0);
    let tono = 0.56 + 0.16 * surco + 0.1 * piedrita + 0.06 * (hash(id) - 0.5);
    v3(tono, tono * 0.98, tono * 0.93)
}

// corteza: grietas verticales profundas
fn tex_corteza(u: f32, v: f32) -> Vec3 {
    let n = fbm(u, v, 4, 4, 111);
    let g = (2.0 * PI * (u * 10.0 + n * 1.5)).sin().abs();
    let grieta = 1.0 - suavizar(0.0, 0.35, g);
    let c = v3(0.34, 0.25, 0.18) * (0.8 + 0.3 * valor2(u * 32.0, v * 4.0, 32, 113));
    c.lerp(v3(0.10, 0.07, 0.05), grieta * 0.8)
}

fn funcion_textura(nombre: &str) -> fn(f32, f32) -> Vec3 {
    match nombre {
        "wood.png" => tex_madera,
        "stone.png" => tex_piedra,
        "metal.png" => tex_metal,
        "water.png" => tex_agua,
        "glass.png" => tex_vidrio,
        "roof_tiles.png" => tex_tejas,
        "paper.png" => tex_papel,
        "leaves.png" => tex_follaje,
        "grass.png" => tex_pasto,
        "gravel.png" => tex_grava,
        _ => tex_corteza,
    }
}

// ---------------------------------------------------------------------
// cielo de atardecer para el skybox
// ---------------------------------------------------------------------

// fbm sin periodo (el cielo no necesita repetirse)
fn fbm_libre(x: f32, y: f32, octavas: u32, semilla: u32) -> f32 {
    let (mut suma, mut amp, mut total, mut f) = (0.0, 0.5, 0.0, 1.0);
    for o in 0..octavas {
        suma += amp * valor2(x * f, y * f, 0, semilla + o * 13);
        total += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    suma / total
}

fn angulo_envuelto(a: f32) -> f32 {
    let mut a = a;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

pub fn cielo(d: Vec3) -> Vec3 {
    let d = d.normalizado();
    let sol = skybox::direccion_sol();
    let e = d.y;
    let az = d.x.atan2(d.z);

    // que tan del lado del sol esta esta direccion (solo en horizontal)
    let horiz = v3(d.x, 0.0, d.z).normalizado();
    let sol_h = v3(sol.x, 0.0, sol.z).normalizado();
    let lado_sol = (horiz.dot(sol_h) * 0.5 + 0.5).powf(2.0);

    let horizonte = v3(0.62, 0.36, 0.50).lerp(v3(1.0, 0.56, 0.30), lado_sol);
    let medio = v3(0.62, 0.36, 0.52).lerp(v3(0.86, 0.46, 0.46), lado_sol);
    let alto = v3(0.22, 0.17, 0.40);
    let cenit = v3(0.06, 0.06, 0.18);

    let mut c;
    if e >= 0.0 {
        c = horizonte.lerp(medio, suavizar(0.0, 0.12, e));
        c = c.lerp(alto, suavizar(0.08, 0.42, e));
        c = c.lerp(cenit, suavizar(0.4, 0.95, e));

        // estrellas que empiezan a asomar arriba
        let q = |x: f32| (x * 700.0).floor() as i32;
        let h = hash2(q(d.x) * 31 + q(d.z), q(d.y), 7);
        if h > 0.9975 {
            c += Vec3::UNO * (0.7 * suavizar(0.45, 0.8, e));
        }

        // nubes alargadas, iluminadas de rosa/naranja del lado del sol
        if e > 0.015 {
            let k = 0.45 / (e + 0.12);
            let n = fbm_libre(d.x * k * 0.7, d.z * k * 2.6, 5, 17);
            let dens = suavizar(0.46, 0.7, n) * suavizar(0.015, 0.08, e) * (1.0 - suavizar(0.3, 0.55, e));
            let luz = (lado_sol.powf(1.5) * 0.8 + 0.2 * n).clamp(0.0, 1.0);
            let color_nube = v3(0.36, 0.25, 0.40).lerp(v3(1.0, 0.64, 0.46), luz);
            c = c.lerp(color_nube, dens * 0.85);
        }
    } else {
        // debajo del horizonte: un mar de nubes violeta que se oscurece
        let t = suavizar(0.0, 0.5, -e);
        c = horizonte.lerp(v3(0.14, 0.10, 0.22), t * 0.9 + 0.1);
        let n = fbm_libre(d.x * 3.0 / (-e + 0.1), d.z * 3.0 / (-e + 0.1), 4, 29);
        c = c * (0.85 + 0.3 * n);
    }

    // montanas lejanas y el monte Fuji detras del templo (direccion -Z)
    let n1 = fbm_libre(az * 4.0, 0.5, 6, 41);
    let cordillera = 0.02 + 0.09 * n1 * n1;
    let n2 = fbm_libre(az * 9.0, 1.7, 5, 43);
    let cerros = 0.008 + 0.035 * n2 * n2;
    let da = angulo_envuelto(az - PI);
    let fuji = 0.17 * (1.0 - da.abs() / 0.5).max(0.0).powf(1.5);
    let fuji = fuji.min(0.155);
    if e > -0.02 {
        if e < cerros {
            c = v3(0.16, 0.11, 0.20).lerp(horizonte, 0.15);
        } else if e < fuji.max(cordillera) {
            // mas claras abajo (bruma) y un poco mas oscuras en las cumbres
            let lejos = v3(0.30, 0.22, 0.38);
            c = lejos.lerp(horizonte, 0.45 - 2.0 * e.max(0.0));
            if fuji > cordillera {
                let linea_nieve = fuji * (0.66 + 0.1 * valor2(da * 60.0, 3.0, 0, 47));
                if e > linea_nieve {
                    c = v3(0.96, 0.74, 0.78); // nieve con el rosa del atardecer
                }
            }
        }
    }

    // el sol y su resplandor
    let cs = d.dot(sol).max(0.0);
    c += v3(1.0, 0.55, 0.28) * (cs.powf(6.0) * 0.35 + cs.powf(60.0) * 0.6);
    let disco = suavizar(0.9990, 0.9996, cs);
    c = c.lerp(v3(1.0, 0.93, 0.75), disco);
    c
}

// ---------------------------------------------------------------------

fn faltan(carpeta: &str, nombres: &[&str]) -> bool {
    nombres.iter().any(|n| !Path::new(&format!("{carpeta}/{n}")).exists())
}

// genera todo lo que falte (o todo, si forzar = true)
pub fn asegurar_assets(forzar: bool) {
    if forzar || faltan(CARPETA_TEXTURAS, &ARCHIVOS) {
        std::fs::create_dir_all(CARPETA_TEXTURAS).expect("no se pudo crear la carpeta de texturas");
        for nombre in ARCHIVOS {
            println!("generando {CARPETA_TEXTURAS}/{nombre}");
            let f = funcion_textura(nombre);
            guardar(&format!("{CARPETA_TEXTURAS}/{nombre}"), LADO_TEXTURA, &f);
        }
    }
    if forzar || faltan(CARPETA_SKYBOX, &CARAS) {
        std::fs::create_dir_all(CARPETA_SKYBOX).expect("no se pudo crear la carpeta del skybox");
        for (cara, nombre) in CARAS.iter().enumerate() {
            println!("generando {CARPETA_SKYBOX}/{nombre}");
            guardar(&format!("{CARPETA_SKYBOX}/{nombre}"), LADO_CIELO, &|u, v| {
                cielo(skybox::direccion_de_cara(cara, u * 2.0 - 1.0, v * 2.0 - 1.0))
            });
        }
    }
}
