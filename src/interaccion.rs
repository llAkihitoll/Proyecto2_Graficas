// estado de las interacciones del diorama y sus animaciones. todas las
// interacciones estan disponibles desde el inicio y en cualquier orden.

use crate::ambiente::{Ambiente, DURACION_DIA};
use crate::figuras::Interactivo;
use crate::matematica::suavizar;

// cuanto dura la animacion de la campana (s)
const DURACION_CAMPANA: f32 = 7.0;
// momento en que el mazo golpea la campana (s)
const T_GOLPE: f32 = 0.3;

pub struct EstadoDiorama {
    pub campana_activada: bool,
    pub linternas_encendidas: bool,
    pub fuente_activa: bool,
    pub puerta_abierta: bool,

    // animaciones
    pub t_campana: f32,
    pub apertura_puerta: f32, // 0 cerrada .. 1 abierta
    pub nivel_fuente: f32,    // 0 seca .. 1 llena
    pub tiempo: f32,          // reloj del agua

    pub mensaje: String,
    pub t_mensaje: f32,

    // hora del dia, estacion y ciclo dia/noche
    pub ambiente: Ambiente,
}

impl Default for EstadoDiorama {
    fn default() -> Self {
        EstadoDiorama {
            campana_activada: false,
            linternas_encendidas: false,
            fuente_activa: false,
            puerta_abierta: false,
            t_campana: DURACION_CAMPANA,
            apertura_puerta: 0.0,
            nivel_fuente: 0.0,
            tiempo: 0.0,
            mensaje: String::from("Bienvenido al templo. Explora el jardin libremente."),
            t_mensaje: 6.0,
            ambiente: Ambiente::default(),
        }
    }
}

impl EstadoDiorama {
    // --- ambiente: estacion y hora ---

    pub fn cambiar_estacion(&mut self) {
        self.ambiente.estacion = self.ambiente.estacion.siguiente();
        let texto = format!("Llega {}.", self.ambiente.estacion.nombre().to_lowercase());
        self.avisar(&texto);
    }

    pub fn alternar_ciclo(&mut self) {
        self.ambiente.ciclo_activo = !self.ambiente.ciclo_activo;
        self.avisar(if self.ambiente.ciclo_activo { "El tiempo avanza: ciclo de dia y noche." } else { "Tiempo en pausa." });
    }

    // adelanta o atrasa la hora (horas, puede ser negativo)
    pub fn mover_hora(&mut self, horas: f32) {
        self.ambiente.hora = (self.ambiente.hora + horas).rem_euclid(24.0);
    }

    fn avisar(&mut self, texto: &str) {
        self.mensaje = texto.to_string();
        self.t_mensaje = 4.5;
    }

    // devuelve true si cambio algo que hay que volver a renderizar
    pub fn interactuar(&mut self, que: Interactivo) -> bool {
        match que {
            Interactivo::Ninguno => false,
            Interactivo::Campana => {
                self.t_campana = 0.0;
                self.campana_activada = true;
                self.avisar("Gooong... la campana resuena en el jardin.");
                true
            }
            Interactivo::Linterna => {
                self.linternas_encendidas = !self.linternas_encendidas;
                self.avisar(if self.linternas_encendidas { "Linternas encendidas." } else { "Linternas apagadas." });
                true
            }
            Interactivo::Fuente => {
                self.fuente_activa = !self.fuente_activa;
                self.avisar(if self.fuente_activa { "El agua de la fuente fluye." } else { "La fuente se detiene." });
                true
            }
            Interactivo::Puerta => {
                self.puerta_abierta = !self.puerta_abierta;
                self.avisar(if self.puerta_abierta { "La puerta se abre y revela el altar." } else { "La puerta se cierra." });
                true
            }
        }
    }

    // avanza las animaciones. devuelve (hubo_cambio, suave): "suave" es
    // cuando solo se mueven el agua, el cielo o las particulas; ahi se
    // puede renderizar con mas calidad que durante movimientos grandes
    pub fn actualizar(&mut self, dt: f32) -> (bool, bool) {
        let dt = dt.min(0.1);
        self.t_mensaje = (self.t_mensaje - dt).max(0.0);
        let mut transitorio = false;

        if self.t_campana < DURACION_CAMPANA {
            self.t_campana += dt;
            transitorio = true;
        }

        let objetivo_puerta = if self.puerta_abierta { 1.0 } else { 0.0 };
        if self.apertura_puerta != objetivo_puerta {
            let paso = dt * 0.6;
            self.apertura_puerta = if self.apertura_puerta < objetivo_puerta {
                (self.apertura_puerta + paso).min(1.0)
            } else {
                (self.apertura_puerta - paso).max(0.0)
            };
            transitorio = true;
        }

        let objetivo_fuente = if self.fuente_activa { 1.0 } else { 0.0 };
        if self.nivel_fuente != objetivo_fuente {
            let paso = dt * 0.45;
            self.nivel_fuente = if self.nivel_fuente < objetivo_fuente {
                (self.nivel_fuente + paso).min(1.0)
            } else {
                (self.nivel_fuente - paso).max(0.0)
            };
            transitorio = true;
        }

        let agua = self.fuente_activa || self.nivel_fuente > 0.0;
        if agua {
            self.tiempo += dt;
        }

        let ciclo = self.ambiente.ciclo_activo;
        if ciclo {
            self.mover_hora(dt * 24.0 / DURACION_DIA);
            self.ambiente.t_particulas += dt;
        }

        let suave = agua || ciclo;
        (transitorio || suave, suave && !transitorio)
    }

    // --- valores de la animacion de la campana ---

    // angulo de balanceo: oscilacion amortiguada despues del golpe
    pub fn angulo_campana(&self) -> f32 {
        let t = self.t_campana - T_GOLPE;
        if t <= 0.0 || self.t_campana >= DURACION_CAMPANA {
            return 0.0;
        }
        // el mazo empuja la campana hacia -X, alejandola (con rot_z, un
        // angulo negativo mueve la parte de abajo hacia -X)
        -0.2 * (3.2 * t).sin() * (-t / 2.2).exp()
    }

    // el mazo se acerca hasta tocar la campana y despues vuelve
    pub fn desplazamiento_mazo(&self) -> f32 {
        let reposo = 0.14;
        let t = self.t_campana;
        if t >= DURACION_CAMPANA {
            return reposo;
        }
        if t < T_GOLPE {
            reposo - (reposo + 0.03) * suavizar(0.0, T_GOLPE, t)
        } else {
            let tr = t - T_GOLPE;
            reposo - (reposo + 0.03) * (-tr * 2.5).exp() * (1.0 + 0.3 * (6.0 * tr).sin())
        }
    }

    // brillo del bronce: se enciende con el golpe y se apaga de a poco
    pub fn brillo_campana(&self) -> f32 {
        let t = self.t_campana - T_GOLPE;
        if t <= 0.0 || self.t_campana >= DURACION_CAMPANA {
            return 0.0;
        }
        (-t / 1.4).exp()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cualquier_orden() {
        let mut e = EstadoDiorama::default();
        // la puerta y la fuente funcionan sin tocar antes la campana
        assert!(e.interactuar(Interactivo::Puerta));
        assert!(e.puerta_abierta);
        assert!(e.interactuar(Interactivo::Fuente));
        assert!(e.fuente_activa);
        assert!(e.interactuar(Interactivo::Linterna));
        assert!(e.linternas_encendidas);
        // y se pueden volver a apagar/cerrar
        e.interactuar(Interactivo::Puerta);
        assert!(!e.puerta_abierta);
    }

    #[test]
    fn animaciones_terminan() {
        let mut e = EstadoDiorama::default();
        e.interactuar(Interactivo::Campana);
        for _ in 0..200 {
            e.actualizar(0.05);
        }
        assert_eq!(e.angulo_campana(), 0.0);
        let (animando, _) = e.actualizar(0.05);
        assert!(!animando);
    }
}
