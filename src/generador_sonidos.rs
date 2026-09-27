// genera los efectos de sonido (assets/sounds/*.wav) por sintesis, igual
// que las texturas: el diorama despues los CARGA desde archivo, asi que se
// pueden reemplazar por grabaciones reales sin tocar el codigo.
//
//   campana:      suma de parciales inarmonicos que decaen (como un bonsho)
//                 con pulsaciones, mas el golpe del mazo de madera
//   linterna:     chispa + "fuop" de la llama al encender; soplido al apagar
//   fuente:       chapoteo al activar, agua corriendo en loop, gorgoteo al parar
//   puerta:       crujido de madera (friccion por pulsos + resonadores) y
//                 golpe al cerrar
//
// los .wav se escriben a mano (PCM de 16 bits, mono, 44.1 kHz).

use crate::ruido::hash;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub const CARPETA_SONIDOS: &str = "assets/sounds";
pub const ARCHIVOS_SONIDO: [&str; 8] = [
    "campana.wav",
    "linterna_encender.wav",
    "linterna_apagar.wav",
    "fuente_activar.wav",
    "fuente_detener.wav",
    "fuente_agua.wav",
    "puerta_abrir.wav",
    "puerta_cerrar.wav",
];

const FM: f32 = 44_100.0; // frecuencia de muestreo

// ---------------------------------------------------------------------
// utilidades
// ---------------------------------------------------------------------

// ruido blanco reproducible
struct Azar(u32);

impl Azar {
    fn sig(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(1);
        hash(self.0.wrapping_mul(2_654_435_761)) * 2.0 - 1.0
    }
    fn uniforme(&mut self) -> f32 {
        self.sig() * 0.5 + 0.5
    }
}

// filtro pasa bajos de un polo
struct PasaBajos {
    y: f32,
    a: f32,
}

impl PasaBajos {
    fn nuevo(corte: f32) -> Self {
        PasaBajos { y: 0.0, a: 1.0 - (-2.0 * PI * corte / FM).exp() }
    }
    fn paso(&mut self, x: f32) -> f32 {
        self.y += self.a * (x - self.y);
        self.y
    }
}

// resonador de dos polos (pasa banda angosto): suena "a madera" o "a agua"
// segun la frecuencia central
struct Resonador {
    b1: f32,
    b2: f32,
    y1: f32,
    y2: f32,
    g: f32,
}

impl Resonador {
    fn nuevo(frec: f32, ancho: f32) -> Self {
        let r = (-PI * ancho / FM).exp();
        let w = 2.0 * PI * frec / FM;
        Resonador { b1: 2.0 * r * w.cos(), b2: -r * r, y1: 0.0, y2: 0.0, g: 1.0 - r }
    }
    fn paso(&mut self, x: f32) -> f32 {
        let y = self.g * x + self.b1 * self.y1 + self.b2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

fn muestras(segundos: f32) -> usize {
    (segundos * FM) as usize
}

fn normalizar(s: &mut [f32], pico: f32) {
    let max = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if max > 0.0 {
        for x in s.iter_mut() {
            *x *= pico / max;
        }
    }
}

// entrada y salida suaves para que no haya "clicks"
fn bordes(s: &mut [f32], entrada: f32, salida: f32) {
    let n = s.len();
    let ne = muestras(entrada).min(n);
    let ns = muestras(salida).min(n);
    for i in 0..ne {
        s[i] *= i as f32 / ne as f32;
    }
    for i in 0..ns {
        s[n - 1 - i] *= i as f32 / ns as f32;
    }
}

fn escribir_wav(ruta: &str, s: &[f32]) -> std::io::Result<()> {
    let mut f = BufWriter::new(File::create(ruta)?);
    let datos = (s.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + datos).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // tamano del bloque fmt
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&1u16.to_le_bytes())?; // mono
    f.write_all(&(FM as u32).to_le_bytes())?;
    f.write_all(&(FM as u32 * 2).to_le_bytes())?; // bytes por segundo
    f.write_all(&2u16.to_le_bytes())?; // bytes por muestra
    f.write_all(&16u16.to_le_bytes())?; // bits
    f.write_all(b"data")?;
    f.write_all(&datos.to_le_bytes())?;
    for &x in s {
        let v = (x.clamp(-1.0, 1.0) * 32_767.0) as i16;
        f.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------
// sonidos
// ---------------------------------------------------------------------

// campana de templo (bonsho): grave, larga y con pulsaciones ("uaaa-uaaa")
fn campana() -> Vec<f32> {
    let n = muestras(7.0);
    let f0 = 96.0;
    // (relacion con f0, amplitud, tiempo de decaimiento en s)
    let parciales = [
        (0.5, 0.55, 6.0),
        (1.0, 1.0, 5.0),
        (1.183, 0.5, 3.8),
        (1.506, 0.45, 3.0),
        (2.0, 0.35, 2.4),
        (2.514, 0.28, 1.8),
        (2.662, 0.24, 1.5),
        (3.011, 0.2, 1.2),
        (4.166, 0.12, 0.8),
        (5.43, 0.08, 0.5),
    ];
    let mut az = Azar(11);
    let mut golpe = PasaBajos::nuevo(900.0);
    let mut s = vec![0.0; n];
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        let ataque = 1.0 - (-t / 0.004).exp();
        let mut v = 0.0;
        for (k, &(rel, amp, tau)) in parciales.iter().enumerate() {
            let f = f0 * rel;
            // cada parcial va con una copia apenas desafinada: las dos
            // frecuencias se "pelean" y producen la pulsacion tipica
            let desafino = 0.35 + 0.1 * k as f32;
            v += amp * (-t / tau).exp() * ((2.0 * PI * f * t).sin() + 0.8 * (2.0 * PI * (f + desafino) * t).sin());
        }
        // golpe del mazo de madera: ruido grave muy corto y un "tum"
        let tum = (2.0 * PI * 62.0 * t).sin() * (-t / 0.09).exp() * 0.9;
        let ruido = golpe.paso(az.sig()) * (-t / 0.025).exp() * 2.5;
        *x = v * ataque * 0.35 + tum + ruido;
    }
    normalizar(&mut s, 0.9);
    bordes(&mut s, 0.0, 0.3);
    s
}

// encender una linterna: chispazo del fosforo y la llama que prende
fn linterna_encender() -> Vec<f32> {
    let n = muestras(1.1);
    let mut az = Azar(21);
    let mut agudo = PasaBajos::nuevo(6000.0);
    let mut grave = PasaBajos::nuevo(500.0);
    let mut s = vec![0.0; n];
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        let r = az.sig();
        // chispa: ruido brillante en los primeros 60 ms
        let a = agudo.paso(r);
        let chispa = (r - a) * (-t / 0.02).exp() * if t < 0.07 { 1.0 } else { 0.0 };
        // la llama: un soplido grave que sube y se estabiliza
        let fuop = grave.paso(r) * ((t - 0.05).max(0.0) / 0.12).min(1.0) * (-(t - 0.2).max(0.0) / 0.45).exp() * 3.0;
        // chasquidos sueltos del fuego
        let crepita = if az.uniforme() > 0.9993 { az.sig() * 0.8 } else { 0.0 };
        *x = chispa * 0.9 + fuop + crepita * (t / 0.2).min(1.0);
    }
    normalizar(&mut s, 0.7);
    bordes(&mut s, 0.002, 0.2);
    s
}

// apagar una linterna: un soplido suave
fn linterna_apagar() -> Vec<f32> {
    let n = muestras(0.65);
    let mut az = Azar(31);
    let mut f1 = PasaBajos::nuevo(1400.0);
    let mut f2 = PasaBajos::nuevo(1400.0);
    let mut s = vec![0.0; n];
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        let env = (t / 0.06).min(1.0) * (-(t - 0.06).max(0.0) / 0.16).exp();
        *x = f2.paso(f1.paso(az.sig())) * env;
    }
    normalizar(&mut s, 0.5);
    bordes(&mut s, 0.005, 0.1);
    s
}

// burbuja: un tono corto que sube de frecuencia (asi suena una gota o una
// burbuja al reventar en el agua)
fn burbuja(s: &mut [f32], inicio: usize, f_ini: f32, dur: f32, amp: f32) {
    let len = muestras(dur);
    let mut fase = 0.0;
    for k in 0..len {
        if inicio + k >= s.len() {
            break;
        }
        let u = k as f32 / len as f32;
        let f = f_ini * (1.0 + 1.6 * u);
        fase += 2.0 * PI * f / FM;
        s[inicio + k] += amp * fase.sin() * (1.0 - u).powi(2);
    }
}

// agua corriendo: ruido filtrado con variaciones lentas y burbujas.
// "segundos" de largo; con "loop" el final se funde con el principio
fn agua(segundos: f32, semilla: u32, densidad_burbujas: f32) -> Vec<f32> {
    let n = muestras(segundos);
    let mut az = Azar(semilla);
    let mut lp = PasaBajos::nuevo(2500.0);
    let mut hp = PasaBajos::nuevo(300.0);
    let mut s = vec![0.0; n];
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        let r = lp.paso(az.sig());
        let banda = r - hp.paso(r);
        let ondula = 0.7 + 0.3 * (2.0 * PI * 0.7 * t).sin() * (2.0 * PI * 1.9 * t).sin();
        *x = banda * ondula;
    }
    let cantidad = (segundos * densidad_burbujas) as usize;
    for _ in 0..cantidad {
        let ini = (az.uniforme() * n as f32) as usize;
        let f = 400.0 + 1100.0 * az.uniforme();
        burbuja(&mut s, ini, f, 0.015 + 0.03 * az.uniforme(), 0.25 + 0.3 * az.uniforme());
    }
    s
}

fn fuente_activar() -> Vec<f32> {
    let mut s = agua(1.6, 41, 40.0);
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        // el chorro arranca de golpe y se asienta
        *x *= (t / 0.08).min(1.0) * (0.55 + 0.45 * (-t / 0.5).exp());
    }
    normalizar(&mut s, 0.7);
    bordes(&mut s, 0.01, 0.5);
    s
}

fn fuente_detener() -> Vec<f32> {
    let mut s = agua(0.9, 51, 25.0);
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        *x *= (-t / 0.3).exp();
    }
    // ultimo gorgoteo mas grave
    let len = s.len();
    burbuja(&mut s, len / 3, 220.0, 0.06, 0.6);
    normalizar(&mut s, 0.55);
    bordes(&mut s, 0.005, 0.2);
    s
}

// loop del agua de la fuente: se funde el final con el principio para que
// al repetirse no se note el corte
fn fuente_agua_loop() -> Vec<f32> {
    let largo = 4.0;
    let fundido = 0.4;
    let mut s = agua(largo + fundido, 61, 22.0);
    let nf = muestras(fundido);
    let n = muestras(largo);
    for k in 0..nf {
        let u = k as f32 / nf as f32;
        s[k] = s[k] * u + s[n + k] * (1.0 - u);
    }
    s.truncate(n);
    normalizar(&mut s, 0.6);
    s
}

// crujido de madera: la friccion de la bisagra es una serie de pulsos
// (se pega y se suelta) cuya frecuencia va cambiando; los pulsos excitan
// resonadores que le dan el timbre de madera
fn crujido(s: &mut [f32], inicio: f32, dur: f32, f_ini: f32, f_fin: f32, semilla: u32) {
    let mut az = Azar(semilla);
    let mut r1 = Resonador::nuevo(620.0, 90.0);
    let mut r2 = Resonador::nuevo(1350.0, 160.0);
    let mut r3 = Resonador::nuevo(2900.0, 400.0);
    let ini = muestras(inicio);
    let len = muestras(dur);
    let mut fase = 0.0;
    for k in 0..len {
        if ini + k >= s.len() {
            break;
        }
        let u = k as f32 / len as f32;
        // la frecuencia sube y baja un poco, con temblor
        let f = f_ini + (f_fin - f_ini) * u + 18.0 * (2.0 * PI * 3.0 * u).sin() + 6.0 * az.sig();
        fase += f / FM;
        let pulso = if fase >= 1.0 {
            fase -= 1.0;
            0.6 + 0.4 * az.uniforme()
        } else {
            0.0
        };
        let env = (u / 0.1).min(1.0) * ((1.0 - u) / 0.2).min(1.0);
        let v = r1.paso(pulso) * 1.0 + r2.paso(pulso) * 0.7 + r3.paso(pulso) * 0.25;
        s[ini + k] += v * env;
    }
}

// golpe seco de madera pesada
fn golpe(s: &mut [f32], inicio: f32, fuerza: f32, semilla: u32) {
    let mut az = Azar(semilla);
    let mut lp = PasaBajos::nuevo(700.0);
    let ini = muestras(inicio);
    for k in 0..muestras(0.45) {
        if ini + k >= s.len() {
            break;
        }
        let t = k as f32 / FM;
        let tum = (2.0 * PI * 72.0 * t).sin() * (-t / 0.12).exp();
        let madera = (2.0 * PI * 210.0 * t).sin() * (-t / 0.05).exp() * 0.5;
        let ruido = lp.paso(az.sig()) * (-t / 0.03).exp() * 2.0;
        s[ini + k] += (tum + madera + ruido) * fuerza;
    }
}

fn puerta_abrir() -> Vec<f32> {
    let mut s = vec![0.0; muestras(2.0)];
    crujido(&mut s, 0.05, 1.5, 95.0, 150.0, 71);
    crujido(&mut s, 0.9, 0.55, 210.0, 120.0, 73);
    golpe(&mut s, 1.55, 0.25, 75);
    normalizar(&mut s, 0.75);
    bordes(&mut s, 0.005, 0.2);
    s
}

fn puerta_cerrar() -> Vec<f32> {
    let mut s = vec![0.0; muestras(1.6)];
    crujido(&mut s, 0.0, 0.8, 150.0, 90.0, 81);
    golpe(&mut s, 0.85, 1.0, 83);
    golpe(&mut s, 0.97, 0.3, 85); // rebote
    normalizar(&mut s, 0.85);
    bordes(&mut s, 0.005, 0.15);
    s
}

fn sintetizar(nombre: &str) -> Vec<f32> {
    match nombre {
        "campana.wav" => campana(),
        "linterna_encender.wav" => linterna_encender(),
        "linterna_apagar.wav" => linterna_apagar(),
        "fuente_activar.wav" => fuente_activar(),
        "fuente_detener.wav" => fuente_detener(),
        "fuente_agua.wav" => fuente_agua_loop(),
        "puerta_abrir.wav" => puerta_abrir(),
        _ => puerta_cerrar(),
    }
}

// genera los sonidos que falten (o todos, si forzar = true)
pub fn asegurar_sonidos(forzar: bool) {
    for nombre in ARCHIVOS_SONIDO {
        let ruta = format!("{CARPETA_SONIDOS}/{nombre}");
        if !forzar && Path::new(&ruta).exists() {
            continue;
        }
        std::fs::create_dir_all(CARPETA_SONIDOS).expect("no se pudo crear la carpeta de sonidos");
        println!("generando {ruta}");
        if let Err(e) = escribir_wav(&ruta, &sintetizar(nombre)) {
            eprintln!("no se pudo guardar {ruta}: {e}");
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sonidos_sin_saturar_y_con_largo_correcto() {
        for nombre in ARCHIVOS_SONIDO {
            let s = sintetizar(nombre);
            assert!(!s.is_empty(), "{nombre} vacio");
            assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "{nombre} satura");
        }
        assert_eq!(fuente_agua_loop().len(), muestras(4.0));
    }
}
