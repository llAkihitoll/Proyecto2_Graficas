// genera los efectos de sonido (assets/sounds/*.wav) por sintesis, igual
// que las texturas: el diorama despues los CARGA desde archivo, asi que se
// pueden reemplazar por grabaciones reales sin tocar el codigo. solo se
// sintetizan los que falten (hoy las linternas, la fuente y la puerta son
// grabaciones; la campana es sintetizada).
//
//   campana:      suma de parciales inarmonicos que decaen (como un bonsho)
//                 con pulsaciones, mas el golpe del mazo de madera
//   linterna:     raspado del fosforo, "fshh" al encender y crepitar de la
//                 llama; soplido y "pff" al apagar
//   fuente:       lluvia de cientos de gotitas por segundo sobre un fondo de
//                 ruido: el chorro arranca, corre en loop y se adelgaza al parar
//   puerta:       pestillo, crujido de bisagra (friccion por pulsos que hace
//                 sonar modos de madera) y golpe del panel al cerrar
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

// ---------------------------------------------------------------------
// piezas sueltas que usan varios sonidos
// ---------------------------------------------------------------------

// pasa altos de un polo (lo que el pasa bajos deja afuera)
struct PasaAltos(PasaBajos);

impl PasaAltos {
    fn nuevo(corte: f32) -> Self {
        PasaAltos(PasaBajos::nuevo(corte))
    }
    fn paso(&mut self, x: f32) -> f32 {
        x - self.0.paso(x)
    }
}

// gota de agua: un tono muy corto que sube rapido de frecuencia (la
// burbujita que queda atrapada al caer la gota vibra asi). cientos de
// estas por segundo son lo que hace que el agua suene a agua
fn gota(s: &mut [f32], inicio: usize, f0: f32, dur: f32, amp: f32) {
    let len = muestras(dur);
    let mut fase = 0.0f32;
    for k in 0..len {
        if inicio + k >= s.len() {
            break;
        }
        let u = k as f32 / len as f32;
        let f = f0 * (1.0 + 1.2 * u * u);
        fase += 2.0 * PI * f / FM;
        let env = (u / 0.08).min(1.0) * (1.0 - u).powi(3);
        s[inicio + k] += amp * fase.sin() * env;
    }
}

// chasquido: rafaga cortisima de ruido que hace sonar un resonador
// (crepitar de la llama, clic de un pestillo)
fn chasquido(s: &mut [f32], inicio: usize, frec: f32, dur: f32, amp: f32, az: &mut Azar) {
    let mut r = Resonador::nuevo(frec, frec * 0.35);
    let len = muestras(dur);
    for k in 0..len.max(1) * 4 {
        if inicio + k >= s.len() {
            break;
        }
        let exc = if k < len { az.sig() * (1.0 - k as f32 / len as f32) } else { 0.0 };
        s[inicio + k] += amp * r.paso(exc) * 6.0;
    }
}

// agua cayendo: fondo de ruido de banda + una lluvia de gotas.
// "densidad(t)" dice cuantas gotas por segundo hay en cada momento y
// "volumen(t)" la envolvente general
fn agua(segundos: f32, semilla: u32, densidad: &dyn Fn(f32) -> f32, volumen: &dyn Fn(f32) -> f32) -> Vec<f32> {
    let n = muestras(segundos);
    let mut az = Azar(semilla);
    let mut s = vec![0.0; n];

    // fondo: el "shhh" continuo del chorro (ruido entre ~600 y ~5000 Hz)
    let mut lp = PasaBajos::nuevo(5000.0);
    let mut hp = PasaAltos::nuevo(600.0);
    let mut lento = 0.0f32;
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        // variacion lenta y aleatoria del caudal
        lento += (az.sig() - lento) * 0.0004;
        let r = hp.paso(lp.paso(az.sig()));
        *x = r * 0.35 * (0.8 + 1.5 * lento) * volumen(t) * (densidad(t) / 400.0).min(1.0);
    }

    // gotas: se reparten en el tiempo segun la densidad
    let paso = 0.001; // se decide cada milisegundo
    let mut t = 0.0;
    while t < segundos {
        let esperadas = densidad(t) * paso;
        if az.uniforme() < esperadas {
            let ini = (t * FM) as usize;
            let f0 = 900.0 + 3200.0 * az.uniforme().powi(2);
            let dur = 0.003 + 0.009 * az.uniforme();
            // la mayoria son chiquitas y de vez en cuando una mas fuerte
            let amp = 0.08 + 0.5 * az.uniforme().powi(4);
            gota(&mut s, ini, f0, dur, amp * volumen(t));
        }
        t += paso;
    }
    s
}

// ---------------------------------------------------------------------
// linternas
// ---------------------------------------------------------------------

// encender: se raspa el fosforo (friccion aspera y aguda), se enciende
// con un "fshh" y la llama queda crepitando un poco
fn linterna_encender() -> Vec<f32> {
    let n = muestras(1.4);
    let mut az = Azar(21);
    let mut s = vec![0.0; n];

    // 1) raspado: ruido agudo cortado en granos irregulares (las
    //    asperezas de la lija), durante ~0.2 s
    let mut banda = Resonador::nuevo(3200.0, 2600.0);
    let mut grano = 0.0f32;
    for i in 0..muestras(0.22) {
        if az.uniforme() < 0.004 {
            grano = 0.4 + 0.6 * az.uniforme();
        }
        grano *= 0.9985;
        let t = i as f32 / FM;
        let env = (t / 0.02).min(1.0) * (1.0 - t / 0.22);
        s[i] += banda.paso(az.sig()) * 1.6 * grano * env;
    }

    // 2) encendido: "fshh" brillante que se abre y se apaga rapido, con
    //    un poco de cuerpo grave debajo
    let ini = muestras(0.2);
    let mut hp = PasaAltos::nuevo(1500.0);
    let mut lp = PasaBajos::nuevo(7000.0);
    let mut cuerpo = PasaBajos::nuevo(350.0);
    for k in 0..muestras(0.6) {
        let t = k as f32 / FM;
        let env = (t / 0.012).min(1.0) * (-t / 0.16).exp();
        let r = az.sig();
        s[ini + k] += hp.paso(lp.paso(r)) * env * 1.3 + cuerpo.paso(r) * env * 2.5;
    }

    // 3) la llama: murmullo suave y chasquidos sueltos que se van espaciando
    let mut llama = PasaBajos::nuevo(900.0);
    for k in 0..n - ini {
        let t = k as f32 / FM;
        s[ini + k] += llama.paso(az.sig()) * 0.9 * (t / 0.1).min(1.0) * (-t / 0.8).exp();
    }
    let mut t = 0.25;
    while t < 1.3 {
        chasquido(&mut s, muestras(t), 1800.0 + 2500.0 * az.uniforme(), 0.0015, 0.9 * (-(t - 0.25) / 0.6).exp(), &mut az);
        t += 0.02 + 0.12 * az.uniforme();
    }

    normalizar(&mut s, 0.7);
    bordes(&mut s, 0.002, 0.25);
    s
}

// apagar: soplido corto (aire, mas agudo que grave) y el "pff" final de
// la llama que se ahoga
fn linterna_apagar() -> Vec<f32> {
    let n = muestras(0.75);
    let mut az = Azar(31);
    let mut s = vec![0.0; n];
    let mut aire = Resonador::nuevo(1800.0, 2200.0);
    let mut hp = PasaAltos::nuevo(500.0);
    for (i, x) in s.iter_mut().enumerate() {
        let t = i as f32 / FM;
        // el soplido sube rapido, se sostiene un instante y se corta
        let env = (t / 0.05).min(1.0) * if t < 0.18 { 1.0 } else { (-(t - 0.18) / 0.07).exp() };
        *x = hp.paso(aire.paso(az.sig())) * env * 4.0;
    }
    // "pff": la llama se ahoga con un golpecito grave
    let ini = muestras(0.16);
    let mut lp = PasaBajos::nuevo(250.0);
    for k in 0..muestras(0.25) {
        let t = k as f32 / FM;
        s[ini + k] += lp.paso(az.sig()) * 2.0 * (t / 0.005).min(1.0) * (-t / 0.05).exp();
    }
    normalizar(&mut s, 0.5);
    bordes(&mut s, 0.005, 0.15);
    s
}

// ---------------------------------------------------------------------
// fuente
// ---------------------------------------------------------------------

// activar: el chorro arranca (la densidad de gotas sube en ~0.4 s) y
// queda corriendo; el final se desvanece para empalmar con el loop
fn fuente_activar() -> Vec<f32> {
    let mut s = agua(
        1.8,
        41,
        &|t| 60.0 + 540.0 * ((t / 0.4).min(1.0)),
        &|t| (t / 0.05).min(1.0) * (1.0 + 0.5 * (-t / 0.3).exp()),
    );
    normalizar(&mut s, 0.65);
    bordes(&mut s, 0.01, 0.6);
    s
}

// detener: el chorro se va adelgazando y quedan unas gotas sueltas
fn fuente_detener() -> Vec<f32> {
    let mut s = agua(1.6, 51, &|t| 500.0 * (-t / 0.25).exp() + 3.0, &|t| (-t / 0.45).exp() * 0.9 + 0.1);
    // ultimas gotas grandes y espaciadas
    let mut az = Azar(53);
    for (k, t) in [0.55f32, 0.8, 1.02, 1.3].iter().enumerate() {
        gota(&mut s, muestras(*t), 700.0 + 400.0 * az.uniforme(), 0.018, 0.5 - 0.08 * k as f32);
    }
    normalizar(&mut s, 0.55);
    bordes(&mut s, 0.005, 0.15);
    s
}

// loop del agua corriendo: denso y parejo. el final se funde con el
// principio para que al repetirse no se note el corte
fn fuente_agua_loop() -> Vec<f32> {
    let largo = 4.0;
    let fundido = 0.4;
    let mut s = agua(largo + fundido, 61, &|_| 600.0, &|_| 1.0);
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

// ---------------------------------------------------------------------
// puerta
// ---------------------------------------------------------------------

// crujido de bisagra: friccion "pega y suelta" (pulsos) a una frecuencia
// que cambia, con pulsos que a veces fallan (tartamudeo) y amplitud muy
// irregular. los pulsos hacen sonar varios modos de la madera
fn crujido(s: &mut [f32], inicio: f32, dur: f32, f_ini: f32, f_fin: f32, amp: f32, semilla: u32) {
    let mut az = Azar(semilla);
    let mut modos = [
        (Resonador::nuevo(480.0, 60.0), 1.0),
        (Resonador::nuevo(930.0, 90.0), 0.8),
        (Resonador::nuevo(1650.0, 140.0), 0.55),
        (Resonador::nuevo(2750.0, 260.0), 0.3),
    ];
    let ini = muestras(inicio);
    let len = muestras(dur);
    let mut fase = 0.0f32;
    let mut presion = 1.0f32;
    for k in 0..len {
        if ini + k >= s.len() {
            break;
        }
        let u = k as f32 / len as f32;
        // la presion de la mano cambia de a poco: modula volumen y tono
        presion += (az.sig() * 0.5 + 0.5 - presion) * 0.0006;
        let f = (f_ini + (f_fin - f_ini) * u) * (0.85 + 0.3 * presion);
        fase += f / FM;
        let mut pulso = 0.0;
        if fase >= 1.0 {
            fase -= 1.0;
            // 1 de cada 6 pulsos se "saltea": da el tartamudeo del crujido
            if az.uniforme() > 0.16 {
                pulso = (0.3 + 0.7 * az.uniforme()) * presion;
            }
        }
        let env = (u / 0.06).min(1.0) * ((1.0 - u) / 0.15).min(1.0);
        let mut v = 0.0;
        for (m, g) in modos.iter_mut() {
            v += m.paso(pulso) * *g;
        }
        s[ini + k] += v * env * amp;
    }
}

// golpe de un panel de madera pesado: varios modos graves que se apagan
// rapido, mas el ruido del impacto
fn golpe(s: &mut [f32], inicio: f32, fuerza: f32, semilla: u32) {
    let mut az = Azar(semilla);
    let mut lp = PasaBajos::nuevo(1200.0);
    let modos = [(95.0f32, 1.0f32, 0.16f32), (168.0, 0.7, 0.1), (290.0, 0.5, 0.07), (470.0, 0.35, 0.045), (760.0, 0.2, 0.03)];
    let ini = muestras(inicio);
    for k in 0..muestras(0.5) {
        if ini + k >= s.len() {
            break;
        }
        let t = k as f32 / FM;
        let mut v = 0.0;
        for (f, a, tau) in modos {
            v += a * (2.0 * PI * f * t).sin() * (-t / tau).exp();
        }
        let impacto = lp.paso(az.sig()) * (-t / 0.012).exp() * 2.5;
        s[ini + k] += (v + impacto) * fuerza;
    }
}

// clic metalico del pestillo
fn pestillo(s: &mut [f32], inicio: f32, amp: f32, semilla: u32) {
    let mut az = Azar(semilla);
    chasquido(s, muestras(inicio), 3400.0, 0.001, amp, &mut az);
    chasquido(s, muestras(inicio + 0.035), 2600.0, 0.001, amp * 0.6, &mut az);
}

fn puerta_abrir() -> Vec<f32> {
    let mut s = vec![0.0; muestras(2.0)];
    pestillo(&mut s, 0.02, 0.15, 71);
    // crujido largo que sube de tono mientras la puerta gira
    crujido(&mut s, 0.12, 1.35, 180.0, 420.0, 12.0, 73);
    // un segundo quejido mas agudo a mitad de camino
    crujido(&mut s, 0.7, 0.45, 520.0, 380.0, 6.0, 75);
    // la puerta llega al tope suavemente
    golpe(&mut s, 1.5, 0.18, 77);
    normalizar(&mut s, 0.75);
    bordes(&mut s, 0.003, 0.2);
    s
}

fn puerta_cerrar() -> Vec<f32> {
    let mut s = vec![0.0; muestras(1.6)];
    // crujido que baja de tono mientras se cierra, mas rapido
    crujido(&mut s, 0.0, 0.7, 400.0, 190.0, 10.0, 81);
    // golpe seco del panel contra el marco y un pequeno rebote
    golpe(&mut s, 0.72, 1.0, 83);
    golpe(&mut s, 0.8, 0.25, 85);
    pestillo(&mut s, 0.74, 0.2, 87);
    normalizar(&mut s, 0.85);
    bordes(&mut s, 0.003, 0.15);
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
