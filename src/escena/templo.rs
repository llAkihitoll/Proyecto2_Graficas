// el templo: salon principal de dos niveles sobre una base de piedra.
//
//   base de piedra (2 escalones) + escalera frontal con pasamanos
//   veranda de madera con barandal perimetral
//   12 columnas bermellon, vigas horizontales (nageshi, kashiragi)
//   frente: 2 paneles shoji con celosia + puerta doble con travesano de vidrio
//   costados: ventanas de vidrio con marco y celosia
//   mensulas (tokyo) sobre las columnas sosteniendo el alero
//   techo inferior de faldon + piso superior + techo principal curvo
//   decoraciones: placa, caja de ofrendas, campanita con cuerda, altar
//
// todo esta escrito en coordenadas del mundo; el templo mira hacia +Z
// (hacia el estanque y la entrada del jardin).

use super::techo::{friso, Techo};
use crate::ambiente::Paleta;
use crate::figuras::*;
use crate::material::*;
use crate::matematica::{suavizar, v3, Mat3};
use std::f32::consts::FRAC_PI_2;

// centro del salon en Z y altura del piso de la veranda
pub const Z0: f32 = -7.0;
pub const Y_PISO: f32 = 1.25;
// altura donde terminan las columnas del primer nivel
const Y_CAB: f32 = 4.1;
// posiciones de las columnas (4 x 3)
const XS: [f32; 4] = [-3.3, -1.1, 1.1, 3.3];
const ZS: [f32; 3] = [Z0 + 2.0, Z0, Z0 - 2.0];
const ZF: f32 = Z0 + 2.0; // pared del frente
const ZB: f32 = Z0 - 2.0; // pared de atras

// piso superior
const XU: f32 = 2.3;
const ZUF: f32 = Z0 + 1.3;
const ZUB: f32 = Z0 - 1.3;

pub fn techo_inferior() -> Techo {
    Techo {
        cx: 0.0,
        cz: Z0,
        y_alero: 4.55,
        ax: 5.4,
        az: 4.1,
        wx: 3.1,
        wz: 2.8,
        alto: 1.35,
        curva: 1.6,
        levante: 0.38,
        nx: 40,
        nz: 30,
        anillo: true,
        tejas: TEJAS,
    }
}

pub fn techo_superior() -> Techo {
    Techo {
        cx: 0.0,
        cz: Z0,
        y_alero: 7.35,
        ax: 4.3,
        az: 3.2,
        wx: 3.2,
        wz: 3.2,
        alto: 2.3,
        curva: 1.7,
        levante: 0.55,
        nx: 36,
        nz: 26,
        anillo: false,
        tejas: TEJAS,
    }
}

// zona interior del salon (ahi casi no llega la luz del cielo)
pub fn interior() -> Aabb {
    Aabb { min: v3(-3.25, Y_PISO - 0.01, ZB + 0.05), max: v3(3.25, Y_CAB, ZF - 0.05) }
}

pub fn crear_templo(v: &mut Vec<Objeto>, paleta: &Paleta) {
    crear_base(v);
    crear_escaleras(v);
    crear_veranda(v);
    crear_barandales(v);
    crear_columnas(v);
    crear_muros(v);
    crear_ventanas(v);
    crear_vigas(v);
    crear_mensulas(v, &perimetro(&XS, &ZS), 4.1, 0.8, 0.12);
    crear_piso_superior(v);
    crear_techos(v, paleta);
    crear_decoraciones(v);
    crear_altar(v);
}

// columnas del borde (las que llevan mensula hacia afuera)
fn perimetro(xs: &[f32], zs: &[f32]) -> Vec<(f32, f32)> {
    let mut p = Vec::new();
    for &x in xs {
        for &z in zs {
            let borde_x = x == xs[0] || x == xs[xs.len() - 1];
            let borde_z = z == zs[0] || z == zs[zs.len() - 1];
            if borde_x || borde_z {
                p.push((x, z));
            }
        }
    }
    p
}

fn crear_base(v: &mut Vec<Objeto>) {
    v.push(caja_mm(v3(-5.2, 0.0, -10.6), v3(5.2, 0.5, -3.4), PIEDRA));
    v.push(caja_mm(v3(-4.6, 0.5, -10.1), v3(4.6, 1.05, -3.9), PIEDRA));
    // cornisa que sobresale un poco sobre el segundo escalon
    v.push(caja_mm(v3(-4.72, 0.98, -10.22), v3(4.72, 1.1, -3.95), PIEDRA));
}

fn crear_escaleras(v: &mut Vec<Objeto>) {
    let n = 6;
    let paso = Y_PISO / n as f32;
    for k in 1..=n {
        let tope = paso * k as f32;
        let frente = -4.2 + (n + 1 - k) as f32 * 0.34;
        v.push(caja_mm(v3(-1.3, 0.0, -4.2), v3(1.3, tope, frente), PIEDRA));
    }
    // pasamanos lacados con remates dorados (giboshi)
    for sg in [-1.0f32, 1.0] {
        let x = sg * 1.42;
        v.push(cilindro(v3(x, 0.0, -2.05), 0.07, 0.95, LACA_ROJA));
        v.push(esfera(v3(x, 1.0, -2.05), 0.1, ORO));
        v.push(viga(v3(x, 0.9, -2.05), v3(x, 1.98, -4.3), 0.09, 0.09, LACA_ROJA));
    }
}

fn crear_veranda(v: &mut Vec<Objeto>) {
    v.push(caja_mm(v3(-4.3, 1.1, -9.9), v3(4.3, Y_PISO, -4.2), MADERA));
    v.push(caja_mm(v3(-4.36, 1.02, -9.96), v3(4.36, 1.15, -4.14), MADERA_OSCURA));
}

// barandal de la veranda: postes, pasamanos arriba y travesano al medio
fn crear_barandales(v: &mut Vec<Objeto>) {
    let (y0, y1) = (Y_PISO, 1.98);
    let poste = |x: f32, z: f32| caja(v3(x, (y0 + y1) * 0.5, z), v3(0.045, (y1 - y0) * 0.5, 0.045), LACA_ROJA);

    // tramos: (inicio, fin) en el plano XZ
    let tramos = [
        (v3(-4.2, 0.0, -4.3), v3(-1.45, 0.0, -4.3)),
        (v3(1.45, 0.0, -4.3), v3(4.2, 0.0, -4.3)),
        (v3(-4.2, 0.0, -4.3), v3(-4.2, 0.0, -9.8)),
        (v3(4.2, 0.0, -4.3), v3(4.2, 0.0, -9.8)),
        (v3(-4.2, 0.0, -9.8), v3(4.2, 0.0, -9.8)),
    ];
    for (a, b) in tramos {
        let largo = (b - a).largo();
        let postes = (largo / 1.1).round().max(1.0) as usize;
        for k in 0..=postes {
            let p = a.lerp(b, k as f32 / postes as f32);
            v.push(poste(p.x, p.z));
        }
        for (y, grosor) in [(1.94, 0.05), (1.58, 0.03)] {
            let min = v3(a.x.min(b.x), y - grosor, a.z.min(b.z)) - v3(0.05, 0.0, 0.05);
            let max = v3(a.x.max(b.x), y + grosor, a.z.max(b.z)) + v3(0.05, 0.0, 0.05);
            v.push(caja_mm(min, max, LACA_ROJA));
        }
    }
    // remates dorados en los postes junto a la escalera y en las esquinas
    for x in [-4.2f32, -1.45, 1.45, 4.2] {
        v.push(esfera(v3(x, 2.03, -4.3), 0.07, ORO));
    }
}

fn crear_columnas(v: &mut Vec<Objeto>) {
    for &x in &XS {
        for &z in &ZS {
            v.push(cilindro(v3(x, Y_PISO, z), 0.15, Y_CAB - Y_PISO, LACA_ROJA));
            // base lacada en negro y anillo dorado arriba
            v.push(cilindro(v3(x, Y_PISO, z), 0.2, 0.14, LACA_NEGRA));
            v.push(cilindro(v3(x, 3.72, z), 0.165, 0.07, ORO));
        }
    }
}

fn crear_muros(v: &mut Vec<Objeto>) {
    // --- frente: dos paneles shoji a los lados de la puerta ---
    for (x0, x1) in [(-3.15f32, -1.25f32), (1.25, 3.15)] {
        v.push(caja_mm(v3(x0, Y_PISO, ZF - 0.05), v3(x1, 1.75, ZF + 0.05), MADERA));
        v.push(caja_mm(v3(x0, 1.75, ZF - 0.02), v3(x1, 3.3, ZF + 0.02), PAPEL_SHOJI));
        // celosia (kumiko) de madera oscura
        for k in 1..=3 {
            let x = x0 + (x1 - x0) * k as f32 / 4.0;
            v.push(caja_mm(v3(x - 0.02, 1.75, ZF - 0.05), v3(x + 0.02, 3.3, ZF + 0.05), MADERA_OSCURA));
            let y = 1.75 + 1.55 * k as f32 / 4.0;
            v.push(caja_mm(v3(x0, y - 0.02, ZF - 0.05), v3(x1, y + 0.02, ZF + 0.05), MADERA_OSCURA));
        }
        // panel alto (ranma) de yeso
        v.push(caja_mm(v3(x0, 3.42, ZF - 0.04), v3(x1, 3.95, ZF + 0.04), YESO));
    }

    // --- marco de la puerta: umbral y travesano de vidrio con barrotes ---
    v.push(caja_mm(v3(-0.95, Y_PISO, ZF - 0.07), v3(0.95, 1.32, ZF + 0.07), MADERA_OSCURA));
    v.push(caja_mm(v3(-0.95, 3.42, ZF - 0.02), v3(0.95, 3.95, ZF + 0.02), VIDRIO));
    for k in 1..=4 {
        let x = -0.95 + 1.9 * k as f32 / 5.0;
        v.push(caja_mm(v3(x - 0.025, 3.42, ZF - 0.05), v3(x + 0.025, 3.95, ZF + 0.05), MADERA_OSCURA));
    }

    // --- costados: tramo de atras de tablas de madera ---
    for sg in [-1.0f32, 1.0] {
        let x = sg * 3.3;
        v.push(caja_mm(v3(x - 0.05, Y_PISO, ZB + 0.15), v3(x + 0.05, 3.95, Z0 - 0.15), MADERA));
    }

    // --- pared de atras ---
    v.push(caja_mm(v3(-3.15, Y_PISO, ZB - 0.05), v3(3.15, 3.95, ZB + 0.05), MADERA));

    // --- cielorraso ---
    v.push(caja_mm(v3(-3.3, Y_CAB, ZB), v3(3.3, 4.2, ZF), MADERA_OSCURA));
}

// ventanas laterales: yeso abajo y arriba, vidrio en el medio con marco
// y celosia. el vidrio deja ver (refractado) el interior del templo
fn crear_ventanas(v: &mut Vec<Objeto>) {
    let (z0, z1) = (Z0 + 0.15, ZF - 0.15);
    for sg in [-1.0f32, 1.0] {
        let x = sg * 3.3;
        v.push(caja_mm(v3(x - 0.05, Y_PISO, z0), v3(x + 0.05, 2.2, z1), YESO));
        v.push(caja_mm(v3(x - 0.02, 2.2, z0), v3(x + 0.02, 3.2, z1), VIDRIO));
        v.push(caja_mm(v3(x - 0.05, 3.2, z0), v3(x + 0.05, 3.95, z1), YESO));
        // marco
        for (ya, yb) in [(2.15f32, 2.24f32), (3.16, 3.25)] {
            v.push(caja_mm(v3(x - 0.08, ya, z0), v3(x + 0.08, yb, z1), MADERA_OSCURA));
        }
        for (za, zb) in [(z0, z0 + 0.08), (z1 - 0.08, z1)] {
            v.push(caja_mm(v3(x - 0.08, 2.2, za), v3(x + 0.08, 3.2, zb), MADERA_OSCURA));
        }
        // celosia
        for k in 1..=3 {
            let z = z0 + (z1 - z0) * k as f32 / 4.0;
            v.push(caja_mm(v3(x - 0.05, 2.2, z - 0.018), v3(x + 0.05, 3.2, z + 0.018), MADERA_OSCURA));
        }
        v.push(caja_mm(v3(x - 0.05, 2.685, z0), v3(x + 0.05, 2.715, z1), MADERA_OSCURA));
    }
}

// viga horizontal que rodea un rectangulo (x +-hx, z entre zf y zb)
fn anillo_vigas(v: &mut Vec<Objeto>, hx: f32, zf: f32, zb: f32, y0: f32, y1: f32, g: f32, m: Material) {
    v.push(caja_mm(v3(-hx - g, y0, zf - g), v3(hx + g, y1, zf + g), m));
    v.push(caja_mm(v3(-hx - g, y0, zb - g), v3(hx + g, y1, zb + g), m));
    for sg in [-1.0f32, 1.0] {
        v.push(caja_mm(v3(sg * hx - g, y0, zb - g), v3(sg * hx + g, y1, zf + g), m));
    }
}

fn crear_vigas(v: &mut Vec<Objeto>) {
    // nageshi (sobre puertas y ventanas) y kashiragi (cabeza de columnas)
    anillo_vigas(v, 3.3, ZF, ZB, 3.3, 3.42, 0.13, LACA_ROJA);
    anillo_vigas(v, 3.3, ZF, ZB, 3.95, Y_CAB, 0.14, LACA_ROJA);
    // viga baja en costados y atras
    for sg in [-1.0f32, 1.0] {
        v.push(caja_mm(v3(sg * 3.3 - 0.1, 1.7, ZB), v3(sg * 3.3 + 0.1, 1.8, Z0 - 0.15), LACA_ROJA));
    }
    v.push(caja_mm(v3(-3.3, 1.7, ZB - 0.1), v3(3.3, 1.8, ZB + 0.1), LACA_ROJA));
}

// mensulas (tokyo): sobre cada columna del borde un bloque (daito), un
// brazo que sale hacia afuera y un taco en la punta que sostiene la
// viga del alero. las esquinas llevan brazo en las dos direcciones
fn crear_mensulas(v: &mut Vec<Objeto>, columnas: &[(f32, f32)], y: f32, vuelo: f32, alto: f32) {
    let xmax = columnas.iter().fold(0.0f32, |m, c| m.max(c.0.abs()));
    let zmax = columnas.iter().fold(f32::MIN, |m, c| m.max(c.1));
    let zmin = columnas.iter().fold(f32::MAX, |m, c| m.min(c.1));
    for &(x, z) in columnas {
        v.push(caja(v3(x, y + alto, z), v3(0.19, alto, 0.19), LACA_ROJA));
        let mut dirs = Vec::new();
        if (x.abs() - xmax).abs() < 1e-3 {
            dirs.push((x.signum(), 0.0));
        }
        if (z - zmax).abs() < 1e-3 {
            dirs.push((0.0, 1.0));
        }
        if (z - zmin).abs() < 1e-3 {
            dirs.push((0.0, -1.0));
        }
        let yb = y + 2.0 * alto;
        for (dx, dz) in dirs {
            let c = v3(x + dx * vuelo * 0.5, yb + 0.06, z + dz * vuelo * 0.5);
            let m = v3(dx.abs() * vuelo * 0.5 + 0.07, 0.07, dz.abs() * vuelo * 0.5 + 0.07);
            v.push(caja(c, m, MADERA_OSCURA));
            v.push(caja(v3(x + dx * vuelo, yb + 0.2, z + dz * vuelo), v3(0.1, 0.08, 0.1), LACA_ROJA));
        }
    }
    // viga del alero (gagyo) apoyada sobre los tacos
    let yb = y + 2.0 * alto + 0.28;
    let hx = xmax + vuelo;
    let (zf, zb) = (zmax + vuelo, zmin - vuelo);
    anillo_vigas(v, hx, zf, zb, yb, yb + 0.12, 0.06, MADERA_OSCURA);
}

fn crear_piso_superior(v: &mut Vec<Objeto>) {
    let (y0, y1) = (5.5, 7.05);
    let xs = [-XU, 0.0, XU];
    let zs = [ZUF, ZUB];
    for &x in &xs {
        for &z in &zs {
            v.push(cilindro(v3(x, y0, z), 0.13, 7.2 - y0, LACA_ROJA));
        }
    }
    // paredes de yeso
    v.push(caja_mm(v3(-XU, y0, ZUF - 0.05), v3(XU, y1, ZUF + 0.05), YESO));
    v.push(caja_mm(v3(-XU, y0, ZUB - 0.05), v3(XU, y1, ZUB + 0.05), YESO));
    for sg in [-1.0f32, 1.0] {
        v.push(caja_mm(v3(sg * XU - 0.05, y0, ZUB), v3(sg * XU + 0.05, y1, ZUF), YESO));
    }
    // ventanas de celosia del frente
    for xc in [-1.15f32, 1.15] {
        let z = ZUF + 0.05;
        v.push(caja_mm(v3(xc - 0.62, 5.92, z), v3(xc + 0.62, 6.68, z + 0.03), MADERA_OSCURA));
        for k in 0..=5 {
            let x = xc - 0.6 + 1.2 * k as f32 / 5.0;
            v.push(caja_mm(v3(x - 0.025, 5.95, z + 0.02), v3(x + 0.025, 6.65, z + 0.07), LACA_ROJA));
        }
        for y in [5.93f32, 6.66] {
            v.push(caja_mm(v3(xc - 0.66, y - 0.03, z + 0.02), v3(xc + 0.66, y + 0.03, z + 0.08), LACA_ROJA));
        }
    }
    anillo_vigas(v, XU, ZUF, ZUB, 7.05, 7.2, 0.12, LACA_ROJA);
    crear_mensulas(v, &perimetro(&xs, &zs), 7.2, 0.55, 0.08);
}

fn crear_techos(v: &mut Vec<Objeto>, paleta: &Paleta) {
    let inf = Techo { tejas: paleta.tejas, ..techo_inferior() };
    let sup = Techo { tejas: paleta.tejas, ..techo_superior() };
    inf.construir(v);
    sup.construir(v);

    // frisos: tapan el hueco entre lo alto de las paredes y el techo
    let esquinas = |hx: f32, zf: f32, zb: f32| {
        [
            (v3(-hx, 0.0, zf), v3(hx, 0.0, zf)),
            (v3(-hx, 0.0, zb), v3(hx, 0.0, zb)),
            (v3(-hx, 0.0, zb), v3(-hx, 0.0, zf)),
            (v3(hx, 0.0, zb), v3(hx, 0.0, zf)),
        ]
    };
    for (a, b) in esquinas(3.3, ZF, ZB) {
        friso(v, &inf, a, b, 4.2, 0.1, YESO);
    }
    for (a, b) in esquinas(XU, ZUF, ZUB) {
        friso(v, &sup, a, b, 7.2, 0.1, YESO);
    }
}

fn crear_decoraciones(v: &mut Vec<Objeto>) {
    // placa con el nombre del templo (gaku) con borde dorado
    v.push(caja(v3(0.0, 6.3, ZUF + 0.12), v3(0.5, 0.3, 0.04), MADERA_OSCURA));
    v.push(caja(v3(0.0, 6.3, ZUF + 0.08), v3(0.57, 0.37, 0.03), ORO));

    // caja de ofrendas (saisen-bako) arriba de la escalera
    v.push(caja(v3(0.0, Y_PISO + 0.25, -4.55), v3(0.6, 0.25, 0.22), MADERA));
    for k in 0..5 {
        let x = -0.48 + 0.24 * k as f32;
        v.push(caja(v3(x, Y_PISO + 0.52, -4.55), v3(0.03, 0.03, 0.2), MADERA_OSCURA));
    }

    // campanita de la entrada (suzu) con su cuerda
    v.push(esfera(v3(0.0, 3.1, -4.8), 0.14, ORO));
    v.push(barra(v3(0.0, 3.3, -4.8), v3(0.0, 3.2, -4.8), 0.02, CUERDA));
    v.push(barra(v3(0.0, 2.98, -4.8), v3(0.0, 1.95, -4.8), 0.035, CUERDA.con_albedo(v3(0.85, 0.2, 0.15))));
}

// altar al fondo del salon: se ve al abrir la puerta
fn crear_altar(v: &mut Vec<Objeto>) {
    let z = ZB + 0.75;
    v.push(caja_mm(v3(-1.2, Y_PISO, ZB + 0.1), v3(1.2, 1.65, ZB + 1.3), LACA_ROJA));
    v.push(cono(v3(0.0, 1.65, z), 0.42, 0.56, 0.16, ORO));
    v.push(elipsoide(v3(0.0, 2.2, z), v3(0.42, 0.5, 0.34), ORO));
    v.push(esfera(v3(0.0, 2.88, z), 0.2, ORO));
    // aureola: disco dorado detras de la cabeza
    v.push(cilindro(v3(0.0, 2.83, z - 0.25), 0.58, 0.04, ORO).rotado(Mat3::rot_x(FRAC_PI_2)));
    for sg in [-1.0f32, 1.0] {
        v.push(cilindro(v3(sg * 0.85, 1.65, z + 0.3), 0.05, 0.3, CERA));
        v.push(esfera(v3(sg * 0.85, 2.0, z + 0.3), 0.045, LLAMA).sin_sombra());
    }
}

// puertas (parte dinamica): dos hojas que giran sobre sus bisagras hacia
// adentro. "apertura" va de 0 (cerrada) a 1 (abierta)
pub fn crear_puertas(v: &mut Vec<Objeto>, apertura: f32) {
    let angulo = suavizar(0.0, 1.0, apertura) * 1.75;
    for (bisagra_x, sg) in [(-0.95f32, 1.0f32), (0.95, -1.0)] {
        let bisagra = v3(bisagra_x, 0.0, ZF);
        let rot = Mat3::rot_y(sg * angulo);
        // la hoja cerrada va de la bisagra hasta el centro
        let (x0, x1) = if sg > 0.0 { (-0.95, -0.005) } else { (0.005, 0.95) };
        let borde_int = if sg > 0.0 { x1 } else { x0 };
        let borde_ext = if sg > 0.0 { x0 } else { x1 };
        let mut partes = vec![
            caja_mm(v3(x0, 1.32, ZF - 0.03), v3(x1, 3.3, ZF + 0.03), MADERA),
            // marco lacado: travesanos y largueros
            caja_mm(v3(x0, 3.16, ZF - 0.05), v3(x1, 3.3, ZF + 0.05), LACA_ROJA),
            caja_mm(v3(x0, 1.32, ZF - 0.05), v3(x1, 1.5, ZF + 0.05), LACA_ROJA),
            caja_mm(v3(x0, 2.2, ZF - 0.05), v3(x1, 2.3, ZF + 0.05), LACA_ROJA),
            caja_mm(v3(borde_ext.min(borde_ext + sg * 0.1), 1.32, ZF - 0.05), v3(borde_ext.max(borde_ext + sg * 0.1), 3.3, ZF + 0.05), LACA_ROJA),
            caja_mm(v3(borde_int.min(borde_int - sg * 0.1), 1.32, ZF - 0.05), v3(borde_int.max(borde_int + sg * 0.1), 3.3, ZF + 0.05), LACA_ROJA),
            // tirador de bronce
            esfera(v3(borde_int - sg * 0.2, 2.25, ZF + 0.08), 0.06, ORO),
        ];
        // clavos dorados en el travesano del medio
        for k in 0..3 {
            let x = x0 + (x1 - x0) * (k as f32 + 1.0) / 4.0;
            partes.push(esfera(v3(x, 2.25, ZF + 0.055), 0.025, ORO));
        }
        for p in partes {
            v.push(p.girado_en(bisagra, rot).con_id(crate::figuras::Interactivo::Puerta));
        }
    }
}
