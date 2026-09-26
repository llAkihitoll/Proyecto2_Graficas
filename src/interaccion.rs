// estado de las interacciones del diorama y sus animaciones.
//
// la progresion es lineal: cada interaccion se desbloquea al completar la
// anterior (campana -> linternas -> fuente -> puerta). una vez
// desbloqueada, se puede usar las veces que se quiera.

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

    // 0 = tocar la campana, 1 = linternas, 2 = fuente, 3 = puerta, 4 = fin
    pub etapa: u8,

    // animaciones
    pub t_campana: f32,
    pub apertura_puerta: f32, // 0 cerrada .. 1 abierta
    pub nivel_fuente: f32,    // 0 seca .. 1 llena
    pub tiempo: f32,          // reloj del agua

    pub mensaje: String,
    pub t_mensaje: f32,
}

impl Default for EstadoDiorama {
    fn default() -> Self {
        EstadoDiorama {
            campana_activada: false,
            linternas_encendidas: false,
            fuente_activa: false,
            puerta_abierta: false,
            etapa: 0,
            t_campana: DURACION_CAMPANA,
            apertura_puerta: 0.0,
            nivel_fuente: 0.0,
            tiempo: 0.0,
            mensaje: String::from("Bienvenido al templo. Explora el jardin y busca la campana (B)."),
            t_mensaje: 6.0,
        }
    }
}

impl EstadoDiorama {
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
                if self.etapa == 0 {
                    self.etapa = 1;
                    self.avisar("La campana resuena en el jardin... Enciende las linternas (L).");
                } else {
                    self.avisar("Gooong... la campana vuelve a sonar.");
                }
                true
            }
            Interactivo::Linterna => {
                if self.etapa < 1 {
                    self.avisar("Las linternas no responden. Primero haz sonar la campana (B).");
                    return false;
                }
                self.linternas_encendidas = !self.linternas_encendidas;
                if self.linternas_encendidas {
                    if self.etapa == 1 {
                        self.etapa = 2;
                        self.avisar("Las linternas iluminan el camino. Activa la fuente (F).");
                    } else {
                        self.avisar("Linternas encendidas.");
                    }
                } else {
                    self.avisar("Linternas apagadas.");
                }
                true
            }
            Interactivo::Fuente => {
                if self.etapa < 2 {
                    self.avisar(if self.etapa == 0 {
                        "La fuente esta seca. Primero haz sonar la campana (B)."
                    } else {
                        "La fuente esta seca. Primero enciende las linternas (L)."
                    });
                    return false;
                }
                self.fuente_activa = !self.fuente_activa;
                if self.fuente_activa {
                    if self.etapa == 2 {
                        self.etapa = 3;
                        self.avisar("El agua fluye. Ahora abre la puerta del templo (T).");
                    } else {
                        self.avisar("La fuente vuelve a fluir.");
                    }
                } else {
                    self.avisar("La fuente se detiene.");
                }
                true
            }
            Interactivo::Puerta => {
                if self.etapa < 3 {
                    self.avisar("La puerta esta sellada. Completa los pasos anteriores.");
                    return false;
                }
                self.puerta_abierta = !self.puerta_abierta;
                if self.puerta_abierta {
                    if self.etapa == 3 {
                        self.etapa = 4;
                        self.avisar("La puerta se abre y revela el altar. Recorrido completo!");
                    } else {
                        self.avisar("La puerta se abre.");
                    }
                } else {
                    self.avisar("La puerta se cierra.");
                }
                true
            }
        }
    }

    pub fn objetivo(&self) -> &'static str {
        match self.etapa {
            0 => "Objetivo: encuentra la campana y hazla sonar (B)",
            1 => "Objetivo: enciende las linternas (L)",
            2 => "Objetivo: sigue el camino y activa la fuente (F)",
            3 => "Objetivo: abre la puerta del templo (T)",
            _ => "Recorrido completo. Explora libremente",
        }
    }

    pub fn desbloqueado(&self, que: Interactivo) -> bool {
        match que {
            Interactivo::Campana => true,
            Interactivo::Linterna => self.etapa >= 1,
            Interactivo::Fuente => self.etapa >= 2,
            Interactivo::Puerta => self.etapa >= 3,
            Interactivo::Ninguno => false,
        }
    }

    // avanza las animaciones. devuelve (hubo_cambio, solo_agua): si solo
    // se esta moviendo el agua de la fuente se puede renderizar con mas
    // calidad que durante movimientos grandes
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
        (transitorio || agua, agua && !transitorio)
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
    fn progresion_en_orden() {
        let mut e = EstadoDiorama::default();
        // nada se puede usar antes de tocar la campana
        assert!(!e.interactuar(Interactivo::Linterna));
        assert!(!e.interactuar(Interactivo::Fuente));
        assert!(!e.interactuar(Interactivo::Puerta));
        assert!(!e.linternas_encendidas && !e.fuente_activa && !e.puerta_abierta);

        assert!(e.interactuar(Interactivo::Campana));
        assert_eq!(e.etapa, 1);
        assert!(!e.interactuar(Interactivo::Fuente));
        assert!(e.interactuar(Interactivo::Linterna));
        assert!(e.linternas_encendidas);
        assert!(!e.interactuar(Interactivo::Puerta));
        assert!(e.interactuar(Interactivo::Fuente));
        assert!(e.interactuar(Interactivo::Puerta));
        assert_eq!(e.etapa, 4);
        assert!(e.puerta_abierta);
    }

    #[test]
    fn desbloqueo_permanente() {
        let mut e = EstadoDiorama::default();
        e.interactuar(Interactivo::Campana);
        e.interactuar(Interactivo::Linterna);
        // apagar las linternas no vuelve a bloquear la fuente
        e.interactuar(Interactivo::Linterna);
        assert!(!e.linternas_encendidas);
        assert!(e.interactuar(Interactivo::Fuente));
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
