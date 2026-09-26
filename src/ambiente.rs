// ambiente del diorama: hora del dia (ciclo dia/noche) y estacion del ano.
//
// la hora decide donde esta el sol (y la luna), el color de la luz, la luz
// ambiente, la exposicion y que imagenes del skybox se mezclan. la
// estacion decide la "paleta" de la escena: color del follaje, nieve,
// estanque congelado y que particulas caen.

use crate::material::*;
use crate::matematica::{suavizar, v3, Vec3};
use std::f32::consts::PI;

// segundos reales que dura un dia completo con el ciclo automatico
pub const DURACION_DIA: f32 = 120.0;

// momentos del dia que tienen su propio skybox
pub const AMANECER: usize = 0;
pub const DIA: usize = 1;
pub const ATARDECER: usize = 2;
pub const NOCHE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estacion {
    Primavera,
    Verano,
    Otono,
    Invierno,
}

impl Estacion {
    pub fn siguiente(self) -> Estacion {
        match self {
            Estacion::Primavera => Estacion::Verano,
            Estacion::Verano => Estacion::Otono,
            Estacion::Otono => Estacion::Invierno,
            Estacion::Invierno => Estacion::Primavera,
        }
    }

    pub fn desde_indice(i: u32) -> Estacion {
        match i % 4 {
            0 => Estacion::Primavera,
            1 => Estacion::Verano,
            2 => Estacion::Otono,
            _ => Estacion::Invierno,
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            Estacion::Primavera => "Primavera",
            Estacion::Verano => "Verano",
            Estacion::Otono => "Otono",
            Estacion::Invierno => "Invierno",
        }
    }

    // altura maxima del sol al mediodia: alto en verano, bajo en invierno
    fn elevacion_maxima(self) -> f32 {
        match self {
            Estacion::Primavera => 0.95,
            Estacion::Verano => 1.2,
            Estacion::Otono => 0.87,
            Estacion::Invierno => 0.6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Particulas {
    Petalos,
    Luciernagas,
    Hojas,
    Nieve,
}

// materiales y detalles de la escena que cambian con la estacion
pub struct Paleta {
    pub pasto: Material,
    pub tejas: Material,
    pub agua_estanque: Material,
    pub estanque_congelado: bool,
    // None = arbol sin hojas (invierno)
    pub hojas_arce: Option<Material>,
    pub hojas_sakura: Option<Material>,
    pub arbusto: Material,
    pub arbusto_flor: Material,
    pub lotos: bool,
    pub nieve_en_pinos: bool,
    // alfombra de hojas o petalos caidos bajo los arboles
    pub hojarasca: Option<Material>,
    pub particulas: Particulas,
}

impl Paleta {
    pub fn de(estacion: Estacion) -> Paleta {
        match estacion {
            Estacion::Primavera => Paleta {
                pasto: PASTO,
                tejas: TEJAS,
                agua_estanque: AGUA,
                estanque_congelado: false,
                hojas_arce: Some(HOJAS_ARCE_VERDE),
                hojas_sakura: Some(HOJAS_SAKURA),
                arbusto: HOJAS_ARBUSTO,
                arbusto_flor: AZALEA,
                lotos: true,
                nieve_en_pinos: false,
                hojarasca: Some(PETALO),
                particulas: Particulas::Petalos,
            },
            Estacion::Verano => Paleta {
                pasto: PASTO.con_albedo(v3(0.78, 0.92, 0.68)),
                tejas: TEJAS,
                agua_estanque: AGUA,
                estanque_congelado: false,
                hojas_arce: Some(HOJAS_VERANO),
                hojas_sakura: Some(HOJAS_VERANO.con_albedo(v3(0.42, 0.7, 0.32))),
                arbusto: HOJAS_ARBUSTO,
                arbusto_flor: HORTENSIA,
                lotos: true,
                nieve_en_pinos: false,
                hojarasca: None,
                particulas: Particulas::Luciernagas,
            },
            Estacion::Otono => Paleta {
                pasto: PASTO.con_albedo(v3(1.15, 0.95, 0.5)),
                tejas: TEJAS,
                agua_estanque: AGUA,
                estanque_congelado: false,
                hojas_arce: Some(HOJAS_ARCE),
                hojas_sakura: Some(HOJAS_OTONO),
                arbusto: HOJAS_ARBUSTO.con_albedo(v3(0.75, 0.62, 0.28)),
                arbusto_flor: HOJAS_ARCE.con_albedo(v3(0.95, 0.35, 0.15)),
                lotos: true,
                nieve_en_pinos: false,
                hojarasca: Some(HOJAS_ARCE),
                particulas: Particulas::Hojas,
            },
            Estacion::Invierno => Paleta {
                pasto: NIEVE,
                tejas: NIEVE_TECHO,
                agua_estanque: HIELO,
                estanque_congelado: true,
                hojas_arce: None,
                hojas_sakura: None,
                arbusto: NIEVE,
                arbusto_flor: NIEVE,
                lotos: false,
                nieve_en_pinos: true,
                hojarasca: None,
                particulas: Particulas::Nieve,
            },
        }
    }
}

// ---------------------------------------------------------------------
// ciclo dia / noche
// ---------------------------------------------------------------------

// horas en las que cada skybox se ve "puro"; entre dos claves se mezclan
const CLAVES: [(f32, usize); 8] = [
    (0.0, NOCHE),
    (4.8, NOCHE),
    (6.0, AMANECER),
    (7.6, DIA),
    (16.2, DIA),
    (17.8, ATARDECER),
    (19.3, NOCHE),
    (24.0, NOCHE),
];

// valores de cada momento: luz del cielo, rebote del suelo, exposicion
const AMB_CIELO: [Vec3; 4] = [v3(0.24, 0.21, 0.30), v3(0.36, 0.40, 0.50), v3(0.22, 0.19, 0.30), v3(0.05, 0.06, 0.12)];
const AMB_SUELO: [Vec3; 4] = [v3(0.12, 0.09, 0.08), v3(0.18, 0.16, 0.12), v3(0.12, 0.09, 0.07), v3(0.015, 0.015, 0.03)];
const EXPOSICION: [f32; 4] = [1.0, 0.8, 1.0, 1.7];

// (momento a, momento b, t): el cielo es a mezclado con b en proporcion t
pub fn mezcla(hora: f32) -> (usize, usize, f32) {
    let h = hora.rem_euclid(24.0);
    for k in 0..CLAVES.len() - 1 {
        let (h0, a) = CLAVES[k];
        let (h1, b) = CLAVES[k + 1];
        if h >= h0 && h <= h1 {
            let t = suavizar(h0, h1, h);
            return (a, b, t);
        }
    }
    (NOCHE, NOCHE, 0.0)
}

// el sol sale detras del templo (del lado del monte Fuji), pasa por
// delante del templo al mediodia y se pone hacia la entrada del jardin
fn direccion_amanecer() -> Vec3 {
    v3(0.65, 0.0, -0.76).normalizado()
}

pub fn direccion_sol(hora: f32, estacion: Estacion) -> Vec3 {
    let r = direccion_amanecer();
    // horizontal y perpendicular al recorrido: hacia donde se inclina el
    // sol al mediodia (adelante-derecha del templo)
    let s = v3(0.76, 0.0, 0.65).normalizado();
    let e = estacion.elevacion_maxima();
    // angulo del recorrido: 0 al amanecer (6 h), pi al atardecer (18 h)
    let th = (hora - 6.0) / 12.0 * PI;
    let arriba = Vec3::ARRIBA * e.sin() + s * e.cos();
    (r * th.cos() + arriba * th.sin()).normalizado()
}

pub struct Iluminacion {
    pub luz_dir: Vec3,
    pub luz_color: Vec3,
    pub amb_cielo: Vec3,
    pub amb_suelo: Vec3,
    pub sol_dir: Vec3,
    pub sol_color: Vec3,
    pub luna_dir: Vec3,
    pub luna_visible: f32,
    pub noche: f32,
    pub exposicion: f32,
    pub mezcla: (usize, usize, f32),
}

pub fn iluminacion(hora: f32, estacion: Estacion) -> Iluminacion {
    let sol = direccion_sol(hora, estacion);
    let luna = -sol;

    // el sol se apaga justo al llegar al horizonte y se vuelve naranja
    // cuando esta bajo
    let sol_vis = suavizar(0.0, 0.1, sol.y);
    let sol_color = v3(1.45, 0.7, 0.38).lerp(v3(1.3, 1.22, 1.08), suavizar(0.05, 0.55, sol.y)) * sol_vis;
    let luna_vis = suavizar(0.0, 0.12, luna.y);

    // la luz direccional principal es el sol de dia y la luna de noche
    let (luz_dir, luz_color) = if sol.y > 0.0 { (sol, sol_color) } else { (luna, v3(0.30, 0.36, 0.55) * (0.75 * luna_vis)) };

    let (a, b, t) = mezcla(hora);
    let peso_noche = if a == NOCHE { 1.0 - t } else { 0.0 } + if b == NOCHE { t } else { 0.0 };
    Iluminacion {
        luz_dir,
        luz_color,
        amb_cielo: AMB_CIELO[a].lerp(AMB_CIELO[b], t),
        amb_suelo: AMB_SUELO[a].lerp(AMB_SUELO[b], t),
        sol_dir: sol,
        sol_color,
        luna_dir: luna,
        luna_visible: luna_vis,
        noche: peso_noche,
        exposicion: EXPOSICION[a] + (EXPOSICION[b] - EXPOSICION[a]) * t,
        mezcla: (a, b, t),
    }
}

pub fn formato_hora(hora: f32) -> String {
    let h = hora.rem_euclid(24.0);
    let horas = h.floor() as u32;
    let minutos = ((h - h.floor()) * 60.0).floor() as u32;
    format!("{horas:02}:{minutos:02}")
}

// estado del ambiente (lo guarda EstadoDiorama)
pub struct Ambiente {
    pub hora: f32,
    pub estacion: Estacion,
    pub ciclo_activo: bool,
    // reloj de las particulas (solo avanza con el ciclo encendido)
    pub t_particulas: f32,
}

impl Default for Ambiente {
    fn default() -> Self {
        Ambiente { hora: 17.4, estacion: Estacion::Primavera, ciclo_activo: false, t_particulas: 0.0 }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_sol_sale_y_se_pone() {
        for e in [Estacion::Primavera, Estacion::Invierno] {
            assert!(direccion_sol(6.0, e).y.abs() < 1e-4);
            assert!(direccion_sol(18.0, e).y.abs() < 1e-4);
            assert!(direccion_sol(12.0, e).y > 0.5);
            assert!(direccion_sol(0.0, e).y < -0.5);
        }
        // en verano el sol sube mas que en invierno
        assert!(direccion_sol(12.0, Estacion::Verano).y > direccion_sol(12.0, Estacion::Invierno).y);
    }

    #[test]
    fn mezcla_de_cielos() {
        assert_eq!(mezcla(12.0).0, DIA);
        assert_eq!(mezcla(2.0).0, NOCHE);
        let (a, b, _) = mezcla(17.0);
        assert_eq!((a, b), (DIA, ATARDECER));
    }

    #[test]
    fn la_luz_no_salta_al_ponerse_el_sol() {
        // justo antes y despues del horizonte la luz principal es casi nula
        let antes = iluminacion(17.99, Estacion::Primavera).luz_color;
        let despues = iluminacion(18.01, Estacion::Primavera).luz_color;
        assert!(antes.largo() < 0.1 && despues.largo() < 0.1);
    }
}
