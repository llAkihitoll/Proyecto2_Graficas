// sonidos de las interacciones y musica de fondo. raylib solo reproduce
// los archivos; el
// volumen y el paneo los calculamos segun donde esta el objeto respecto a
// la camara: un sonido lejano se oye mas bajo, y uno que esta a la
// derecha de la pantalla suena mas por el parlante derecho.

use crate::camara::Camara;
use crate::escena::jardin::{LINTERNAS, POS_CAMPANA, POS_FUENTE};
use crate::escena::templo::Z0;
use crate::figuras::Interactivo;
use crate::generador_sonidos::CARPETA_SONIDOS;
use crate::interaccion::EstadoDiorama;
use crate::matematica::{v3, Vec3};
use raylib::prelude::*;

pub struct Sonidos<'a> {
    campana: Sound<'a>,
    linterna_encender: Sound<'a>,
    linterna_apagar: Sound<'a>,
    fuente_activar: Sound<'a>,
    fuente_detener: Sound<'a>,
    puerta_abrir: Sound<'a>,
    puerta_cerrar: Sound<'a>,
    // agua corriendo mientras la fuente tiene agua (se repite en loop)
    agua: Music<'a>,
    // musica de fondo opcional (assets/music/fondo.*), en loop
    fondo: Option<Musica<'a>>,
    musica_activa: bool,
}

// la musica y el tramo que realmente suena: muchas canciones traen
// segundos de silencio al principio o al final, y al repetirse quedaria un
// hueco mudo. al llegar a "fin" se vuelve a "inicio"
struct Musica<'a> {
    stream: Music<'a>,
    inicio: f32,
    fin: f32,
}

// busca (en segundos) donde empieza y termina el sonido de un archivo
fn tramo_con_sonido(audio: &RaylibAudio, ruta: &str) -> Option<(f32, f32)> {
    let mut wave = audio.new_wave(ruta).ok()?;
    let fm = wave.sample_rate();
    // a mono y en flotante, asi cada muestra es un instante de tiempo
    wave.format(fm as i32, 32, 1);
    let muestras = wave.load_samples();
    let s: &[f32] = muestras.as_ref();
    let umbral = 0.01;
    let primera = s.iter().position(|x| x.abs() > umbral)?;
    let ultima = s.iter().rposition(|x| x.abs() > umbral)?;
    let duracion = s.len() as f32 / fm as f32;
    // un poquito de margen para no cortar ataques ni colas
    Some(((primera as f32 / fm as f32 - 0.1).max(0.0), (ultima as f32 / fm as f32 + 0.4).min(duracion)))
}

fn cargar_musica<'a>(audio: &'a RaylibAudio) -> Option<Musica<'a>> {
    let ruta = ARCHIVOS_MUSICA
        .iter()
        .map(|n| format!("{CARPETA_MUSICA}/{n}"))
        .find(|ruta| std::path::Path::new(ruta).exists())?;
    let stream = audio.new_music(&ruta).ok()?;
    let largo = stream.get_time_length();
    let (inicio, fin) = tramo_con_sonido(audio, &ruta).unwrap_or((0.0, largo));
    println!("musica de fondo: {ruta} (suena de {inicio:.1} s a {fin:.1} s de {largo:.1} s)");
    Some(Musica { stream, inicio, fin })
}

pub const CARPETA_MUSICA: &str = "assets/music";
// formatos que se prueban, en orden
const ARCHIVOS_MUSICA: [&str; 3] = ["fondo.mp3", "fondo.ogg", "fondo.wav"];
const VOLUMEN_MUSICA: f32 = 0.35;

fn pos_campana() -> Vec3 {
    v3(POS_CAMPANA.0, 2.4, POS_CAMPANA.1)
}

fn pos_fuente() -> Vec3 {
    v3(POS_FUENTE.0, 1.0, POS_FUENTE.1)
}

fn pos_puerta() -> Vec3 {
    v3(0.0, 2.3, Z0 + 2.0)
}

// (volumen, paneo) de un sonido que sale de "fuente" escuchado desde la camara
fn espacial(fuente: Vec3, camara: &Camara) -> (f32, f32) {
    let pos = camara.posicion();
    let hacia = fuente - pos;
    let dist = hacia.largo().max(0.01);
    let volumen = (1.6 / (1.0 + dist * 0.07)).clamp(0.25, 1.0);
    let adelante = (camara.objetivo - pos).normalizado();
    let derecha = adelante.cross(Vec3::ARRIBA).normalizado();
    let lado = (hacia / dist).dot(derecha).clamp(-1.0, 1.0);
    // en raylib el paneo 1.0 es izquierda, 0.0 derecha y 0.5 centro
    (volumen, 0.5 - 0.4 * lado)
}

impl<'a> Sonidos<'a> {
    pub fn cargar(audio: &'a RaylibAudio) -> Result<Sonidos<'a>, String> {
        let sonido = |n: &str| audio.new_sound(&format!("{CARPETA_SONIDOS}/{n}")).map_err(|e| format!("{n}: {e}"));
        Ok(Sonidos {
            campana: sonido("campana.wav")?,
            linterna_encender: sonido("linterna_encender.wav")?,
            linterna_apagar: sonido("linterna_apagar.wav")?,
            fuente_activar: sonido("fuente_activar.wav")?,
            fuente_detener: sonido("fuente_detener.wav")?,
            puerta_abrir: sonido("puerta_abrir.wav")?,
            puerta_cerrar: sonido("puerta_cerrar.wav")?,
            agua: audio
                .new_music(&format!("{CARPETA_SONIDOS}/fuente_agua.wav"))
                .map_err(|e| format!("fuente_agua.wav: {e}"))?,
            fondo: cargar_musica(audio),
            musica_activa: true,
        })
    }

    pub fn hay_musica(&self) -> bool {
        self.fondo.is_some()
    }

    pub fn musica_activa(&self) -> bool {
        self.musica_activa
    }

    // tecla M: silencia o vuelve a poner la musica
    pub fn alternar_musica(&mut self) {
        self.musica_activa = !self.musica_activa;
        if let Some(m) = &self.fondo {
            if self.musica_activa {
                m.stream.resume_stream();
            } else {
                m.stream.pause_stream();
            }
        }
    }

    fn tocar(sonido: &Sound, fuente: Vec3, camara: &Camara) {
        let (volumen, paneo) = espacial(fuente, camara);
        sonido.set_volume(volumen);
        sonido.set_pan(paneo);
        sonido.play();
    }

    // se llama justo despues de una interaccion que cambio el estado
    pub fn reproducir(&self, que: Interactivo, estado: &EstadoDiorama, camara: &Camara) {
        match que {
            Interactivo::Campana => {
                self.campana.set_pitch(0.97 + 0.06 * ((estado.tiempo * 7.3).sin() * 0.5 + 0.5));
                Self::tocar(&self.campana, pos_campana(), camara);
            }
            Interactivo::Linterna => {
                // suena desde la linterna mas cercana a la camara
                let pos = camara.posicion();
                let cercana = LINTERNAS
                    .iter()
                    .map(|&(x, z)| v3(x, 1.5, z))
                    .min_by(|a, b| (*a - pos).largo().total_cmp(&(*b - pos).largo()))
                    .unwrap_or(Vec3::CERO);
                let s = if estado.linternas_encendidas { &self.linterna_encender } else { &self.linterna_apagar };
                Self::tocar(s, cercana, camara);
            }
            Interactivo::Fuente => {
                let s = if estado.fuente_activa { &self.fuente_activar } else { &self.fuente_detener };
                Self::tocar(s, pos_fuente(), camara);
            }
            Interactivo::Puerta => {
                let s = if estado.puerta_abierta { &self.puerta_abrir } else { &self.puerta_cerrar };
                Self::tocar(s, pos_puerta(), camara);
            }
            Interactivo::Ninguno => {}
        }
    }

    // cada frame: el agua de la fuente suena mientras tenga agua, mas
    // fuerte cuanto mas llena este
    pub fn actualizar(&self, estado: &EstadoDiorama, camara: &Camara) {
        if let Some(m) = &self.fondo {
            if self.musica_activa && !m.stream.is_stream_playing() {
                m.stream.set_volume(VOLUMEN_MUSICA);
                m.stream.play_stream();
                m.stream.seek_stream(m.inicio);
            }
            // al terminar la parte con sonido, vuelve a empezar sin el silencio
            if m.stream.get_time_played() >= m.fin {
                m.stream.seek_stream(m.inicio);
            }
            m.stream.update_stream();
        }

        let nivel = estado.nivel_fuente;
        if nivel > 0.02 {
            if !self.agua.is_stream_playing() {
                self.agua.play_stream();
            }
            let (volumen, paneo) = espacial(pos_fuente(), camara);
            self.agua.set_volume(volumen * nivel * 0.8);
            self.agua.set_pan(paneo);
            self.agua.update_stream();
        } else if self.agua.is_stream_playing() {
            self.agua.stop_stream();
        }
    }
}
