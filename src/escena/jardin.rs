// el jardin alrededor del templo. la composicion va de adelante (+Z) hacia
// atras (-Z):
//
//   torii (entrada) -> camino de piedras -> 2 linternas -> puente arqueado
//   sobre el estanque -> patio de grava con el campanario (izquierda) y la
//   fuente (derecha) -> 2 linternas al pie de la escalera -> templo
//
// la vegetacion (pinos, arces, sakura, bambu, arbustos) queda en los
// bordes del diorama para enmarcar el templo sin taparlo.

use crate::ambiente::{Paleta, Particulas};
use crate::interaccion::EstadoDiorama;
use crate::ruido::hash;
use crate::figuras::*;
use crate::material::*;
use crate::matematica::{v3, Mat3, Vec3};
use crate::render::Luz;
use std::f32::consts::{FRAC_PI_2, PI};

pub const POS_CAMPANA: (f32, f32) = (-6.4, -1.3);
pub const POS_FUENTE: (f32, f32) = (6.3, -1.1);
pub const LINTERNAS: [(f32, f32); 4] = [(-2.3, -2.6), (2.3, -2.6), (-1.9, 7.4), (1.9, 7.4)];
// linternas de papel colgadas del alero del templo
const CHOCHIN: [(f32, f32); 2] = [(-2.2, -4.25), (2.2, -4.25)];

// estanque
const EST_X: f32 = 7.0;
const EST_Z0: f32 = 1.8;
const EST_Z1: f32 = 5.8;
pub const Y_AGUA: f32 = -0.18;

// puente
const PUENTE_Z0: f32 = 1.0;
const PUENTE_Z1: f32 = 6.6;

fn altura_puente(z: f32) -> f32 {
    let zc = 0.5 * (PUENTE_Z0 + PUENTE_Z1);
    let medio = 0.5 * (PUENTE_Z1 - PUENTE_Z0);
    0.05 + 0.95 * (1.0 - ((z - zc) / medio).powi(2))
}

pub fn crear_jardin(v: &mut Vec<Objeto>, paleta: &Paleta) {
    crear_terreno(v, paleta);
    crear_estanque(v, paleta);
    crear_puente(v);
    crear_camino(v);
    crear_torii(v);
    crear_campanario(v, paleta);
    crear_fuente(v);
    for (x, z) in LINTERNAS {
        crear_linterna(v, x, z);
    }
    crear_vegetacion(v, paleta);
}

// ---------------------------------------------------------------------
// terreno: una "isla" flotante con capas de tierra, pasto arriba y un
// hueco donde va el estanque
// ---------------------------------------------------------------------
fn crear_terreno(v: &mut Vec<Objeto>, paleta: &Paleta) {
    v.push(caja_mm(v3(-12.0, -1.8, -12.0), v3(12.0, -0.6, 12.0), TIERRA));
    v.push(caja_mm(v3(-11.3, -2.7, -11.3), v3(11.3, -1.8, 11.3), TIERRA.con_albedo(v3(0.55, 0.45, 0.38))));
    // pasto en 4 piezas alrededor del estanque
    v.push(caja_mm(v3(-12.0, -0.6, -12.0), v3(12.0, 0.0, EST_Z0), paleta.pasto));
    v.push(caja_mm(v3(-12.0, -0.6, EST_Z1), v3(12.0, 0.0, 12.0), paleta.pasto));
    v.push(caja_mm(v3(-12.0, -0.6, EST_Z0), v3(-EST_X, 0.0, EST_Z1), paleta.pasto));
    v.push(caja_mm(v3(EST_X, -0.6, EST_Z0), v3(12.0, 0.0, EST_Z1), paleta.pasto));
    // patio de grava rastrillada frente al templo y franja del camino
    v.push(caja_mm(v3(-8.6, 0.0, -3.3), v3(8.6, 0.03, 1.55), GRAVA));
    v.push(caja_mm(v3(-1.05, 0.0, 6.05), v3(1.05, 0.03, 11.9), GRAVA));
}

// ---------------------------------------------------------------------
// estanque: lecho de grava, agua (refracta y refleja), borde de piedra,
// rocas, peces koi y hojas de loto
// ---------------------------------------------------------------------
fn crear_estanque(v: &mut Vec<Objeto>, paleta: &Paleta) {
    v.push(caja_mm(v3(-EST_X, -0.62, EST_Z0), v3(EST_X, -0.55, EST_Z1), LECHO));
    // el agua se mete un poquito dentro de las paredes y del lecho, asi
    // un rayo refractado siempre choca primero con el fondo o el borde.
    // en invierno es hielo: sin ondas, pero igual refracta
    let ondas = if paleta.estanque_congelado { 0.0 } else { 0.06 };
    v.push(
        caja_mm(v3(-EST_X - 0.03, -0.58, EST_Z0 - 0.03), v3(EST_X + 0.03, Y_AGUA, EST_Z1 + 0.03), paleta.agua_estanque)
            .con_relieve(Relieve::Ondas { cx: 0.0, cz: 0.0, amplitud: ondas, tiempo: 0.0, radial: false }),
    );
    // borde de piedra
    v.push(caja_mm(v3(-EST_X, -0.6, EST_Z0), v3(-EST_X + 0.1, 0.04, EST_Z1), PIEDRA));
    v.push(caja_mm(v3(EST_X - 0.1, -0.6, EST_Z0), v3(EST_X, 0.04, EST_Z1), PIEDRA));
    v.push(caja_mm(v3(-EST_X, -0.6, EST_Z0), v3(EST_X, 0.04, EST_Z0 + 0.1), PIEDRA));
    v.push(caja_mm(v3(-EST_X, -0.6, EST_Z1 - 0.1), v3(EST_X, 0.04, EST_Z1), PIEDRA));

    // rocas: en la orilla (medio enterradas) y un par dentro del agua
    let rocas = [
        (-6.6, 0.0, 2.2, 0.55, 0.35, 0.45, 0.3),
        (-5.2, 0.0, 1.95, 0.45, 0.28, 0.35, 1.1),
        (-3.4, 0.0, 2.0, 0.5, 0.3, 0.4, 2.0),
        (3.1, 0.0, 1.95, 0.45, 0.3, 0.4, 0.7),
        (4.9, 0.0, 2.05, 0.6, 0.35, 0.45, 2.6),
        (6.5, 0.0, 3.1, 0.5, 0.4, 0.6, 0.2),
        (6.4, 0.0, 5.3, 0.55, 0.3, 0.45, 1.5),
        (4.2, 0.0, 5.65, 0.5, 0.3, 0.4, 0.9),
        (-2.8, 0.0, 5.7, 0.45, 0.28, 0.4, 2.2),
        (-4.9, 0.0, 5.6, 0.6, 0.35, 0.45, 0.4),
        (-6.6, 0.0, 4.6, 0.5, 0.4, 0.55, 1.8),
        (-2.2, -0.25, 3.3, 0.45, 0.4, 0.38, 0.6),
        (2.9, -0.28, 4.2, 0.55, 0.45, 0.42, 1.3),
    ];
    for (x, y, z, rx, ry, rz, giro) in rocas {
        v.push(elipsoide(v3(x, y, z), v3(rx, ry, rz), ROCA).rotado(Mat3::rot_y(giro)));
        if paleta.nieve_en_pinos {
            // gorro de nieve sobre la roca
            v.push(elipsoide(v3(x, y + ry * 0.62, z), v3(rx * 0.8, ry * 0.45, rz * 0.8), NIEVE).rotado(Mat3::rot_y(giro)));
        }
    }

    // peces koi bajo el agua (se ven gracias a la refraccion)
    let peces = [
        (-4.5, 3.2, 0.4, KOI_NARANJA),
        (-3.7, 4.5, 2.1, KOI_BLANCO),
        (3.9, 3.0, -0.8, KOI_NARANJA),
        (5.0, 4.6, 2.8, KOI_NARANJA),
        (-5.8, 3.5, 1.2, KOI_BLANCO),
    ];
    for (x, z, giro, m) in peces {
        let r = Mat3::rot_y(giro);
        let c = v3(x, -0.38, z);
        v.push(elipsoide(c, v3(0.3, 0.075, 0.11), m).rotado(r));
        v.push(elipsoide(c + r.por(v3(-0.36, 0.0, 0.0)), v3(0.1, 0.02, 0.11), KOI_NARANJA).rotado(r));
        if m.albedo.x > 0.9 && m.albedo.y > 0.8 {
            // manchas naranjas en el koi blanco
            v.push(esfera(c + r.por(v3(0.08, 0.05, 0.0)), 0.07, KOI_NARANJA));
        }
    }

    if !paleta.lotos {
        return;
    }
    // hojas de loto flotando y un par de flores
    let hojas = [(-5.5, 4.8, 0.45), (-4.8, 5.25, 0.33), (-6.0, 2.8, 0.4), (4.6, 3.3, 0.4), (5.4, 2.6, 0.35), (-2.2, 4.9, 0.3)];
    for (x, z, r) in hojas {
        v.push(cilindro(v3(x, Y_AGUA - 0.01, z), r, 0.025, HOJA_LOTO));
    }
    for (x, z) in [(-5.4, 4.75), (4.6, 3.25)] {
        v.push(elipsoide(v3(x, Y_AGUA + 0.1, z), v3(0.11, 0.1, 0.11), FLOR_LOTO));
        v.push(esfera(v3(x, Y_AGUA + 0.2, z), 0.05, ORO));
    }
}

// ---------------------------------------------------------------------
// puente arqueado (taiko-bashi): tablero de madera en tramos que siguen
// un arco, barandas bermellon y pilotes que se hunden en el agua
// ---------------------------------------------------------------------
fn crear_puente(v: &mut Vec<Objeto>) {
    let n = 8;
    let z_de = |i: usize| PUENTE_Z0 + (PUENTE_Z1 - PUENTE_Z0) * i as f32 / n as f32;
    for i in 0..n {
        let (za, zb) = (z_de(i), z_de(i + 1));
        let a = v3(0.0, altura_puente(za), za);
        let b = v3(0.0, altura_puente(zb), zb);
        v.push(viga(a, b, 1.9, 0.12, MADERA));
        // vigas laterales bajo el tablero
        for sg in [-1.0f32, 1.0] {
            let d = v3(sg * 0.9, -0.1, 0.0);
            v.push(viga(a + d, b + d, 0.1, 0.16, LACA_ROJA));
        }
        for sg in [-1.0f32, 1.0] {
            let x = sg * 0.92;
            for (alto, grosor) in [(0.72, 0.075), (0.38, 0.045)] {
                v.push(viga(v3(x, a.y + alto, za), v3(x, b.y + alto, zb), grosor, grosor, LACA_ROJA));
            }
        }
    }
    for i in (0..=n).step_by(2) {
        let z = z_de(i);
        let y = altura_puente(z);
        for sg in [-1.0f32, 1.0] {
            v.push(cilindro(v3(sg * 0.92, y, z), 0.055, 0.78, LACA_ROJA));
            if i == 0 || i == n {
                v.push(esfera(v3(sg * 0.92, y + 0.84, z), 0.085, ORO));
            }
        }
    }
    // pilotes dentro del agua
    for z in [2.6f32, 5.0] {
        for sg in [-1.0f32, 1.0] {
            v.push(barra(v3(sg * 0.7, -0.58, z), v3(sg * 0.7, altura_puente(z) - 0.1, z), 0.075, MADERA_OSCURA));
        }
    }
}

// ---------------------------------------------------------------------
// camino de losas y piedras de paso
// ---------------------------------------------------------------------
fn crear_camino(v: &mut Vec<Objeto>) {
    let desvio = [0.06f32, -0.05, 0.04, -0.07, 0.03, -0.04, 0.05];
    for (k, d) in desvio.iter().enumerate() {
        let z = 11.4 - 0.75 * k as f32;
        v.push(caja(v3(d * 1.5, 0.05, z), v3(0.55, 0.035, 0.26), PIEDRA).rotado(Mat3::rot_y(*d)));
    }
    for (k, z) in [0.55f32, -0.2, -0.95, -1.7].iter().enumerate() {
        v.push(caja(v3(0.0, 0.065, *z), v3(0.72, 0.04, 0.3), PIEDRA).rotado(Mat3::rot_y(desvio[k] * 0.6)));
    }
    // piedras redondas hacia la campana y hacia la fuente
    for sg in [-1.0f32, 1.0] {
        for k in 0..3 {
            let x = sg * (2.2 + 1.15 * k as f32);
            let z = -0.95 - 0.12 * k as f32 + 0.1 * sg;
            v.push(cilindro(v3(x, 0.0, z), 0.3 - 0.02 * k as f32, 0.07, PIEDRA));
        }
    }
}

// ---------------------------------------------------------------------
// torii: la puerta de entrada al jardin
// ---------------------------------------------------------------------
fn crear_torii(v: &mut Vec<Objeto>) {
    let z = 10.4;
    for sg in [-1.0f32, 1.0] {
        let x = sg * 1.7;
        v.push(cilindro(v3(x, 0.0, z), 0.17, 3.45, LACA_ROJA));
        v.push(cilindro(v3(x, 0.0, z), 0.23, 0.32, LACA_NEGRA));
    }
    v.push(caja(v3(0.0, 2.75, z), v3(2.25, 0.1, 0.08), LACA_ROJA)); // nuki
    v.push(caja(v3(0.0, 3.0, z), v3(0.1, 0.18, 0.07), LACA_ROJA)); // gakuzuka
    v.push(caja(v3(0.0, 2.98, z + 0.07), v3(0.24, 0.17, 0.02), MADERA_OSCURA)); // placa
    v.push(caja(v3(0.0, 3.26, z), v3(2.3, 0.1, 0.12), LACA_ROJA)); // shimaki
    // kasagi: viga negra de arriba, con las puntas curvadas hacia arriba
    v.push(caja(v3(0.0, 3.46, z), v3(1.6, 0.1, 0.17), LACA_NEGRA));
    for sg in [-1.0f32, 1.0] {
        v.push(viga(v3(sg * 1.5, 3.46, z), v3(sg * 2.8, 3.64, z), 0.34, 0.2, LACA_NEGRA));
    }
}

// ---------------------------------------------------------------------
// campanario (shoro): base de piedra, 4 postes, vigas y techo propio.
// la campana y el mazo son dinamicos (ver crear_campana)
// ---------------------------------------------------------------------
fn crear_campanario(v: &mut Vec<Objeto>, paleta: &Paleta) {
    let (bx, bz) = POS_CAMPANA;
    v.push(caja_mm(v3(bx - 1.3, 0.0, bz - 1.3), v3(bx + 1.3, 0.4, bz + 1.3), PIEDRA));
    for (dx, dz) in [(-0.95f32, -0.95f32), (0.95, -0.95), (-0.95, 0.95), (0.95, 0.95)] {
        v.push(cilindro(v3(bx + dx, 0.4, bz + dz), 0.12, 3.0, LACA_ROJA));
        v.push(cilindro(v3(bx + dx, 0.4, bz + dz), 0.16, 0.14, LACA_NEGRA));
    }
    for sg in [-1.0f32, 1.0] {
        // vigas de arriba (a lo largo de Z) y travesanos bajos
        v.push(caja_mm(v3(bx + sg * 0.95 - 0.09, 3.15, bz - 1.2), v3(bx + sg * 0.95 + 0.09, 3.33, bz + 1.2), LACA_ROJA));
        v.push(caja_mm(v3(bx + sg * 0.95 - 0.06, 1.3, bz - 0.95), v3(bx + sg * 0.95 + 0.06, 1.4, bz + 0.95), LACA_ROJA));
    }
    // viga central de donde cuelgan la campana y el mazo
    v.push(caja_mm(v3(bx - 1.2, 3.3, bz - 0.1), v3(bx + 1.68, 3.46, bz + 0.1), MADERA_OSCURA));
    super::techo::Techo {
        cx: bx,
        cz: bz,
        y_alero: 3.55,
        ax: 1.75,
        az: 1.75,
        wx: 1.75,
        wz: 1.75,
        alto: 1.2,
        curva: 1.5,
        levante: 0.25,
        nx: 14,
        nz: 14,
        anillo: false,
        tejas: paleta.tejas,
    }
    .construir(v);
}

// campana de bronce (dinamica). gira alrededor del eje Z, colgada de su
// punto de suspension; el mazo de madera se acerca, golpea y vuelve
fn crear_campana(v: &mut Vec<Objeto>, estado: &EstadoDiorama) {
    let (bx, bz) = POS_CAMPANA;
    let pivote = v3(bx, 3.3, bz);
    let rot = Mat3::rot_z(estado.angulo_campana());
    let brillo = estado.brillo_campana();
    let bronce = METAL.con_emision(v3(1.0, 0.6, 0.25) * (0.7 * brillo));

    let partes = [
        cilindro(v3(bx, 3.1, bz), 0.05, 0.2, bronce),
        cono(v3(bx, 2.85, bz), 0.36, 0.2, 0.25, bronce),
        cono(v3(bx, 1.9, bz), 0.47, 0.36, 0.95, bronce),
        cilindro(v3(bx, 1.86, bz), 0.49, 0.08, bronce),
        cilindro(v3(bx, 2.35, bz), 0.43, 0.05, bronce),
        cilindro(v3(bx, 2.7, bz), 0.39, 0.05, bronce),
        // punto de golpe (tsukiza) mirando hacia el mazo
        cilindro(v3(bx + 0.45, 2.07, bz), 0.1, 0.06, bronce).rotado(Mat3::rot_z(FRAC_PI_2)),
    ];
    for p in partes {
        v.push(p.girado_en(pivote, rot).con_id(Interactivo::Campana));
    }

    // mazo (shumoku) colgado de dos cuerdas
    let d = estado.desplazamiento_mazo();
    v.push(barra(v3(bx + 0.55 + d, 2.1, bz), v3(bx + 1.7 + d, 2.1, bz), 0.1, MADERA).con_id(Interactivo::Campana));
    for x in [0.8f32, 1.5] {
        v.push(barra(v3(bx + x, 3.3, bz), v3(bx + x + d, 2.2, bz), 0.015, CUERDA).con_id(Interactivo::Campana));
    }
}

// ---------------------------------------------------------------------
// fuente de piedra octogonal con una esfera de cristal arriba
// ---------------------------------------------------------------------
fn crear_fuente(v: &mut Vec<Objeto>) {
    let (fx, fz) = POS_FUENTE;
    let id = Interactivo::Fuente;
    let mut partes = vec![
        cilindro(v3(fx, 0.0, fz), 1.55, 0.12, PIEDRA),
        cilindro(v3(fx, 0.12, fz), 1.35, 0.23, PIEDRA),
        cilindro(v3(fx, 0.35, fz), 0.18, 1.15, PIEDRA),
        cono(v3(fx, 1.5, fz), 0.2, 0.62, 0.26, PIEDRA),
        cilindro(v3(fx, 1.76, fz), 0.06, 0.12, METAL),
        cilindro(v3(fx, 1.86, fz), 0.14, 0.03, METAL),
        // esfera de cristal: refracta el jardin dado vuelta
        esfera(v3(fx, 2.2, fz), 0.3, VIDRIO),
    ];
    // paredes octogonales y su borde
    for k in 0..8 {
        let a = k as f32 * PI / 4.0;
        let c = v3(fx + 1.3 * a.cos(), 0.46, fz + 1.3 * a.sin());
        partes.push(caja(c, v3(0.09, 0.34, 0.59), PIEDRA).rotado(Mat3::rot_y(-a)));
        let tapa = v3(fx + 1.3 * a.cos(), 0.83, fz + 1.3 * a.sin());
        partes.push(caja(tapa, v3(0.13, 0.03, 0.62), PIEDRA).rotado(Mat3::rot_y(-a)));
    }
    // monedas y piedritas en el fondo (se ven refractadas)
    let fondo = [(0.5, 0.3), (-0.6, 0.4), (0.2, -0.7), (-0.3, -0.5), (0.8, -0.2), (-0.85, -0.1)];
    for (k, (dx, dz)) in fondo.iter().enumerate() {
        if k % 2 == 0 {
            partes.push(cilindro(v3(fx + dx, 0.35, fz + dz), 0.07, 0.015, ORO));
        } else {
            partes.push(elipsoide(v3(fx + dx, 0.36, fz + dz), v3(0.1, 0.05, 0.08), ROCA));
        }
    }
    for p in partes {
        v.push(p.con_id(id));
    }
}

// agua de la fuente (dinamica): el nivel sube al activarla, aparecen
// ondas circulares y chorros de gotas que caen del cuenco de arriba
fn crear_agua_fuente(v: &mut Vec<Objeto>, estado: &EstadoDiorama) {
    let (fx, fz) = POS_FUENTE;
    let n = estado.nivel_fuente;
    let t = estado.tiempo;
    let tope = 0.42 + 0.33 * n;
    v.push(
        cilindro(v3(fx, 0.3, fz), 1.3, tope - 0.3, AGUA)
            .con_relieve(Relieve::Ondas { cx: fx, cz: fz, amplitud: 0.09 * n, tiempo: t, radial: true })
            .con_id(Interactivo::Fuente),
    );
    if n > 0.05 {
        v.push(
            cilindro(v3(fx, 1.7, fz), 0.56, 0.06 + 0.02 * n, AGUA)
                .con_relieve(Relieve::Ondas { cx: fx, cz: fz, amplitud: 0.06 * n, tiempo: t, radial: true })
                .con_id(Interactivo::Fuente),
        );
    }
    if estado.fuente_activa && n > 0.3 {
        let chorros = 8;
        let gotas = 5;
        for a in 0..chorros {
            let ang = a as f32 * 2.0 * PI / chorros as f32 + 0.2;
            let (s, c) = ang.sin_cos();
            for k in 0..gotas {
                let fase = (t * 1.8 + k as f32 / gotas as f32).fract();
                let tt = fase * 0.46;
                let r = 0.6 + 1.0 * tt;
                let y = 1.76 - 4.9 * tt * tt;
                if y > tope {
                    v.push(esfera(v3(fx + r * c, y, fz + r * s), 0.05, AGUA).sin_sombra().con_id(Interactivo::Fuente));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// linterna de piedra (toro) con caja de fuego de bronce y vidrio
// ---------------------------------------------------------------------
fn crear_linterna(v: &mut Vec<Objeto>, x: f32, z: f32) {
    let mut partes = vec![
        cono(v3(x, 0.0, z), 0.42, 0.32, 0.2, PIEDRA),
        cilindro(v3(x, 0.2, z), 0.13, 0.9, PIEDRA),
        cono(v3(x, 1.1, z), 0.2, 0.38, 0.15, PIEDRA),
        caja(v3(x, 1.27, z), v3(0.27, 0.02, 0.27), METAL),
        caja(v3(x, 1.5, z), v3(0.2, 0.215, 0.2), VIDRIO),
        cilindro(v3(x, 1.7, z), 0.58, 0.035, METAL),
        cono(v3(x, 1.72, z), 0.56, 0.1, 0.26, METAL),
        cono(v3(x, 1.98, z), 0.08, 0.04, 0.1, METAL),
        esfera(v3(x, 2.12, z), 0.08, METAL),
    ];
    for (dx, dz) in [(-0.22f32, -0.22f32), (0.22, -0.22), (-0.22, 0.22), (0.22, 0.22)] {
        partes.push(caja(v3(x + dx, 1.5, z + dz), v3(0.03, 0.22, 0.03), METAL));
    }
    for p in partes {
        v.push(p.con_id(Interactivo::Linterna));
    }
}

// partes que cambian al prender las linternas: la llama de cada linterna
// de piedra y las linternas de papel (chochin) del alero del templo
fn crear_luces_linternas(v: &mut Vec<Objeto>, luces: &mut Vec<Luz>, encendidas: bool) {
    for (x, z) in LINTERNAS {
        let m = if encendidas { LLAMA } else { MECHA };
        v.push(esfera(v3(x, 1.45, z), 0.07, m).sin_sombra().con_id(Interactivo::Linterna));
        if encendidas {
            luces.push(Luz { pos: v3(x, 1.5, z), color: v3(1.0, 0.58, 0.25) * 1.9, radio: 5.5, sombras: false });
        }
    }
    let papel = YESO.con_albedo(v3(1.2, 0.3, 0.2));
    let papel = if encendidas { papel.con_emision(v3(1.3, 0.42, 0.14)) } else { papel };
    for (x, z) in CHOCHIN {
        let partes = [
            barra(v3(x, 4.52, z), v3(x, 4.06, z), 0.012, CUERDA),
            cilindro(v3(x, 3.98, z), 0.12, 0.08, LACA_NEGRA),
            elipsoide(v3(x, 3.7, z), v3(0.22, 0.3, 0.22), papel),
            cilindro(v3(x, 3.36, z), 0.12, 0.07, LACA_NEGRA),
        ];
        for p in partes {
            v.push(p.con_id(Interactivo::Linterna));
        }
        if encendidas {
            luces.push(Luz { pos: v3(x, 3.7, z + 0.35), color: v3(1.0, 0.45, 0.2) * 1.2, radio: 3.5, sombras: false });
        }
    }
}

// ---------------------------------------------------------------------
// vegetacion
// ---------------------------------------------------------------------

// tronco o rama: tronco de cono que va de a hasta b
fn rama(a: Vec3, b: Vec3, r0: f32, r1: f32, m: Material) -> Objeto {
    let mut o = barra(a, b, r0, m);
    if let Forma::Cilindro { mitad_alto, .. } = o.forma {
        o.forma = Forma::Cilindro { r_abajo: r0, r_arriba: r1, mitad_alto };
    }
    o
}

// pino negro japones con la copa podada en "nubes"
fn pino(v: &mut Vec<Objeto>, base: Vec3, alto: f32, incl: Vec3, nieve: bool) {
    let tope = base + v3(incl.x, alto, incl.z);
    v.push(rama(base, tope, 0.28, 0.13, CORTEZA));
    let p1 = base.lerp(tope, 0.55);
    let p2 = base.lerp(tope, 0.78);
    let e1 = p1 + v3(1.4, 0.3, -0.4);
    let e2 = p2 + v3(-1.2, 0.4, 0.6);
    v.push(rama(p1, e1, 0.1, 0.05, CORTEZA));
    v.push(rama(p2, e2, 0.09, 0.05, CORTEZA));
    v.push(elipsoide(tope + v3(0.0, 0.1, 0.0), v3(1.3, 0.42, 1.1), HOJAS_PINO));
    v.push(elipsoide(tope + v3(-0.2, 0.62, 0.1), v3(0.8, 0.3, 0.7), HOJAS_PINO));
    v.push(elipsoide(e1, v3(1.0, 0.32, 0.85), HOJAS_PINO));
    v.push(elipsoide(e2, v3(0.95, 0.3, 0.8), HOJAS_PINO));
    if nieve {
        // la nieve se acumula arriba de cada "nube" de la copa
        for (c, r) in [
            (tope + v3(0.0, 0.3, 0.0), v3(1.15, 0.26, 0.95)),
            (tope + v3(-0.2, 0.78, 0.1), v3(0.68, 0.18, 0.58)),
            (e1 + v3(0.0, 0.18, 0.0), v3(0.85, 0.2, 0.72)),
            (e2 + v3(0.0, 0.17, 0.0), v3(0.8, 0.19, 0.66)),
        ] {
            v.push(elipsoide(c, r, NIEVE));
        }
    }
}

// arbol de copa redonda (arce, sakura). sin hojas (invierno) se ven sus
// ramas desnudas
fn arbol(v: &mut Vec<Objeto>, base: Vec3, alto: f32, ancho: f32, hojas: Option<Material>, incl: Vec3) {
    let tope = base + v3(incl.x, alto, incl.z);
    v.push(rama(base, tope, 0.22, 0.11, CORTEZA));
    let medio = base.lerp(tope, 0.6);
    v.push(rama(medio, medio + v3(0.8, 0.7, 0.3) * ancho, 0.08, 0.04, CORTEZA));
    v.push(rama(medio, medio + v3(-0.7, 0.8, -0.4) * ancho, 0.08, 0.04, CORTEZA));
    let copas = [
        (v3(0.0, 0.35, 0.0), v3(1.15, 0.8, 1.1)),
        (v3(0.9, 0.0, 0.3), v3(0.85, 0.65, 0.8)),
        (v3(-0.8, 0.1, -0.35), v3(0.8, 0.6, 0.85)),
        (v3(0.1, -0.1, 0.85), v3(0.75, 0.55, 0.7)),
    ];
    match hojas {
        Some(m) => {
            for (d, r) in copas {
                v.push(elipsoide(tope + d * ancho, r * ancho, m));
            }
        }
        None => {
            for (d, _) in copas {
                v.push(rama(tope - v3(0.0, 0.3, 0.0), tope + d * (ancho * 1.3), 0.06, 0.02, CORTEZA));
            }
        }
    }
}

// hojas o petalos caidos alrededor del tronco
fn hojarasca(v: &mut Vec<Objeto>, base: Vec3, ancho: f32, m: Material) {
    for (dx, dz, r) in [(0.5f32, 0.3f32, 0.9f32), (-0.6, -0.2, 0.7), (0.1, -0.7, 0.6)] {
        let c = base + v3(dx * ancho, 0.015, dz * ancho);
        v.push(elipsoide(c, v3(r * ancho, 0.025, r * ancho * 0.8), m).rotado(Mat3::rot_y(dx * 3.0)));
    }
}

fn bambu(v: &mut Vec<Objeto>, x: f32, z: f32, alto: f32, incl: f32) {
    let a = v3(x, 0.0, z);
    let b = v3(x + incl, alto, z + incl * 0.4);
    v.push(barra(a, b, 0.055, BAMBU));
    // nudos de la caña
    for k in 1..4 {
        let p = a.lerp(b, k as f32 / 4.0);
        v.push(cilindro(p - v3(0.0, 0.02, 0.0), 0.066, 0.04, BAMBU.con_albedo(v3(0.7, 1.1, 0.5))));
    }
    v.push(elipsoide(b + v3(0.15, -0.3, 0.0), v3(0.55, 0.4, 0.5), HOJAS_ARBUSTO));
}

// arbusto: un volumen central y dos mas chicos a los costados, para que
// no parezca una esfera perfecta
fn arbusto(v: &mut Vec<Objeto>, x: f32, z: f32, r: Vec3, m: Material) {
    v.push(elipsoide(v3(x, r.y * 0.55, z), r * 0.85, m));
    v.push(elipsoide(v3(x + r.x * 0.55, r.y * 0.4, z + r.z * 0.2), r * 0.6, m));
    v.push(elipsoide(v3(x - r.x * 0.4, r.y * 0.38, z - r.z * 0.45), r * 0.62, m));
}

fn crear_vegetacion(v: &mut Vec<Objeto>, paleta: &Paleta) {
    pino(v, v3(-9.6, 0.0, 8.6), 3.2, v3(0.8, 0.0, -0.3), paleta.nieve_en_pinos);
    pino(v, v3(9.3, 0.0, -9.2), 3.6, v3(-0.7, 0.0, 0.4), paleta.nieve_en_pinos);
    let arces = [
        (v3(-9.2, 0.0, -8.2), 3.4, 1.15, v3(0.4, 0.0, 0.2)),
        (v3(9.4, 0.0, 3.6), 2.8, 1.0, v3(-0.3, 0.0, 0.1)),
        (v3(-4.4, 0.0, 9.6), 2.2, 0.75, v3(0.1, 0.0, 0.0)),
    ];
    for (base, alto, ancho, incl) in arces {
        arbol(v, base, alto, ancho, paleta.hojas_arce, incl);
    }
    // sakura inclinado sobre el estanque: se refleja en el agua
    let sakura = v3(-9.0, 0.0, 3.8);
    arbol(v, sakura, 3.0, 1.2, paleta.hojas_sakura, v3(1.3, 0.0, 0.0));
    if let Some(m) = paleta.hojarasca {
        // en primavera caen petalos del sakura, en otono hojas de los arces
        if paleta.particulas == Particulas::Petalos {
            hojarasca(v, sakura, 1.1, m);
        } else {
            for (base, _, ancho, _) in arces {
                hojarasca(v, base, ancho * 1.3, m);
            }
        }
    }

    // bosquecillo de bambu detras de la fuente
    let canas = [
        (9.0, -4.0, 5.0, 0.2),
        (9.6, -4.6, 5.6, -0.1),
        (10.3, -3.8, 4.8, 0.15),
        (9.3, -5.5, 5.3, 0.25),
        (10.1, -5.9, 5.8, -0.2),
        (10.8, -4.9, 5.1, 0.1),
        (8.7, -6.6, 4.6, 0.3),
        (10.6, -6.8, 5.4, -0.15),
        (9.8, -7.4, 4.9, 0.05),
    ];
    for (x, z, h, i) in canas {
        bambu(v, x, z, h, i);
    }

    let verde = paleta.arbusto;
    let flor = paleta.arbusto_flor;
    let arbustos = [
        (-3.9, -2.9, v3(0.55, 0.45, 0.5), verde),
        (3.9, -2.9, v3(0.55, 0.45, 0.5), verde),
        (-5.7, -4.6, v3(0.7, 0.5, 0.6), flor),
        (5.7, -4.9, v3(0.7, 0.5, 0.6), flor),
        (-5.6, -8.9, v3(0.8, 0.6, 0.7), verde),
        (5.8, -9.6, v3(0.75, 0.55, 0.7), flor),
        (-7.8, 6.5, v3(0.8, 0.55, 0.65), flor),
        (7.7, 6.6, v3(0.75, 0.5, 0.6), verde),
        (3.3, 6.6, v3(0.5, 0.38, 0.45), flor),
        (-3.3, 6.5, v3(0.55, 0.4, 0.5), verde),
        (2.8, 11.1, v3(0.6, 0.45, 0.55), verde),
        (-2.9, 11.0, v3(0.6, 0.45, 0.5), flor),
        (-10.6, 0.5, v3(0.7, 0.5, 0.7), verde),
        (10.7, 0.8, v3(0.7, 0.5, 0.6), flor),
    ];
    for (x, z, r, m) in arbustos {
        arbusto(v, x, z, r, m);
    }

    let rocas = [
        (-10.6, -2.6, v3(0.8, 0.5, 0.6), 0.4),
        (10.4, 9.8, v3(0.7, 0.45, 0.55), 1.2),
        (5.8, 10.8, v3(0.55, 0.35, 0.45), 2.0),
        (-6.2, 10.9, v3(0.6, 0.4, 0.5), 0.8),
        (-10.8, 11.0, v3(0.5, 0.35, 0.45), 2.5),
        (0.5, -11.2, v3(0.9, 0.5, 0.6), 0.1),
    ];
    for (x, z, r, g) in rocas {
        v.push(elipsoide(v3(x, r.y * 0.3, z), r, ROCA).rotado(Mat3::rot_y(g)));
        if paleta.nieve_en_pinos {
            v.push(elipsoide(v3(x, r.y * 0.85, z), v3(r.x * 0.8, r.y * 0.4, r.z * 0.8), NIEVE).rotado(Mat3::rot_y(g)));
        }
    }
}

// ---------------------------------------------------------------------
// particulas de cada estacion: petalos (primavera), luciernagas de noche
// (verano), hojas (otono) y nieve (invierno). su posicion sale de un
// hash por particula y del reloj, asi no hace falta guardar estado
// ---------------------------------------------------------------------
fn crear_particulas(v: &mut Vec<Objeto>, estado: &EstadoDiorama, noche: f32) {
    let tipo = Paleta::de(estado.ambiente.estacion).particulas;
    let t = estado.ambiente.t_particulas;
    let h = |i: u32, k: u32| hash(i.wrapping_mul(7919).wrapping_add(k.wrapping_mul(104_729)));
    let tau = 2.0 * PI;

    if tipo == Particulas::Luciernagas {
        if noche < 0.2 {
            return; // de dia no se ven
        }
        for i in 0..26 {
            // alrededor del estanque y de los arbustos, flotando bajito
            let cx = -8.0 + 16.0 * h(i, 1);
            let cz = -1.0 + 9.0 * h(i, 2);
            let x = cx + 0.6 * (t * 0.7 + tau * h(i, 3)).sin();
            let z = cz + 0.6 * (t * 0.5 + tau * h(i, 4)).cos();
            let y = 0.4 + 1.3 * h(i, 5) + 0.25 * (t * 1.3 + tau * h(i, 6)).sin();
            let parpadeo = (0.5 + 0.5 * (t * 3.0 + tau * h(i, 7)).sin()) * noche;
            v.push(esfera(v3(x, y, z), 0.035, LUCIERNAGA.con_emision(LUCIERNAGA.emision * parpadeo)).sin_sombra());
        }
        return;
    }

    let (cantidad, velocidad, radios) = match tipo {
        Particulas::Petalos => (40, 0.5, v3(0.07, 0.012, 0.05)),
        Particulas::Hojas => (36, 0.8, v3(0.09, 0.012, 0.065)),
        _ => (60, 0.6, v3(0.04, 0.04, 0.04)),
    };
    let alto = 9.0;
    for i in 0..cantidad {
        let x0 = -11.0 + 22.0 * h(i, 1);
        let z0 = -11.0 + 22.0 * h(i, 2);
        let caida = t * velocidad * (0.8 + 0.4 * h(i, 4)) + alto * h(i, 3);
        let y = alto - caida.rem_euclid(alto);
        let x = x0 + 0.6 * (t * 0.9 + tau * h(i, 5)).sin();
        let z = z0 + 0.4 * (t * 0.7 + tau * h(i, 6)).cos();
        let m = match tipo {
            Particulas::Petalos => PETALO,
            Particulas::Hojas if i % 2 == 0 => HOJAS_ARCE,
            Particulas::Hojas => HOJAS_OTONO,
            _ => NIEVE,
        };
        // los petalos y hojas dan vueltas mientras caen
        let giro = Mat3::rot_x(t * 2.0 + tau * h(i, 7)).tras(&Mat3::rot_z(t * 1.3 + tau * h(i, 8)));
        v.push(elipsoide(v3(x, y, z), radios, m).rotado(giro).sin_sombra());
    }
}

// todo lo que se reconstruye cuando cambia el estado del diorama
pub fn crear_dinamicos(v: &mut Vec<Objeto>, luces: &mut Vec<Luz>, estado: &EstadoDiorama, noche: f32) {
    crear_campana(v, estado);
    crear_agua_fuente(v, estado);
    crear_luces_linternas(v, luces, estado.linternas_encendidas);
    crear_particulas(v, estado, noche);
}
