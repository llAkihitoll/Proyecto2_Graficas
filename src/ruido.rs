// ruido procedural para generar las texturas y el cielo. todo es
// periodico ("tileable"): la red de valores se envuelve con modulo, asi
// una textura se puede repetir sobre una superficie sin que se note la
// costura.

// hash entero -> [0, 1). variante del hash de "lowbias32"
#[inline]
pub fn hash(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x & 0x00ff_ffff) as f32 / 16_777_216.0
}

#[inline]
pub fn hash2(x: i32, y: i32, semilla: u32) -> f32 {
    hash((x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663) ^ semilla.wrapping_mul(83_492_791))
}

#[inline]
fn suave(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

// ruido de valor 2D con periodo (en celdas). si periodo = 0 no se envuelve
pub fn valor2(x: f32, y: f32, periodo: i32, semilla: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = suave(x - xi as f32);
    let fy = suave(y - yi as f32);
    let w = |v: i32| if periodo > 0 { v.rem_euclid(periodo) } else { v };
    let a = hash2(w(xi), w(yi), semilla);
    let b = hash2(w(xi + 1), w(yi), semilla);
    let c = hash2(w(xi), w(yi + 1), semilla);
    let d = hash2(w(xi + 1), w(yi + 1), semilla);
    let ab = a + (b - a) * fx;
    let cd = c + (d - c) * fx;
    ab + (cd - ab) * fy
}

// "fractal brownian motion": varias octavas de ruido, cada una al doble de
// frecuencia y la mitad de amplitud. u, v en [0,1) para que tile
pub fn fbm(u: f32, v: f32, base: i32, octavas: u32, semilla: u32) -> f32 {
    let mut suma = 0.0;
    let mut amp = 0.5;
    let mut total = 0.0;
    let mut per = base;
    for o in 0..octavas {
        suma += amp * valor2(u * per as f32, v * per as f32, per, semilla + o * 17);
        total += amp;
        amp *= 0.5;
        per *= 2;
    }
    suma / total
}

// ruido celular (Voronoi) periodico: devuelve (distancia al centro mas
// cercano, distancia al segundo, id de la celda). con f2 - f1 se sacan
// los bordes entre celdas (juntas entre piedras)
pub fn voronoi(u: f32, v: f32, celdas: i32, semilla: u32) -> (f32, f32, u32) {
    let x = u * celdas as f32;
    let y = v * celdas as f32;
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    let mut id = 0;
    for j in -1..=1 {
        for i in -1..=1 {
            let cx = xi + i;
            let cy = yi + j;
            let wx = cx.rem_euclid(celdas);
            let wy = cy.rem_euclid(celdas);
            let px = cx as f32 + 0.15 + 0.7 * hash2(wx, wy, semilla);
            let py = cy as f32 + 0.15 + 0.7 * hash2(wx, wy, semilla + 1);
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = (wx * 131 + wy * 7) as u32;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2, id)
}
