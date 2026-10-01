// personaje en miniatura (un peregrino con sombrero de paja) que camina
// por el diorama con las flechas.
//
// no hay fisica: para saber a que altura van los pies se tira un rayo
// hacia abajo desde un poco mas arriba del escalon mas alto que puede
// subir (asi sube la escalera del templo y el puente), y para no
// atravesar paredes, columnas o arboles se tiran rayos horizontales
// hacia donde quiere avanzar, a varias alturas del cuerpo.

use crate::bvh::Bvh;
use crate::escena::Escena;
use crate::figuras::*;
use crate::material::*;
use crate::matematica::{v3, Mat3, Vec3};
use std::f32::consts::PI;

const VELOCIDAD: f32 = 2.2; // m/s
const RADIO: f32 = 0.2;
// lo mas alto que puede subir de un paso (los escalones miden ~0.21)
const ESCALON: f32 = 0.3;
// alturas (sobre los pies) de los rayos de choque: arriba del escalon
const ALTURAS_CHOQUE: [f32; 3] = [0.36, 0.6, 0.85];
// el agua del estanque esta mas abajo: no se puede bajar ahi
const Y_MIN: f32 = -0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Personaje {
    pub pos: Vec3,      // punto entre los pies
    pub rumbo: f32,     // hacia donde mira (angulo sobre Y, 0 = +Z)
    pub fase: f32,      // fase del ciclo de pasos
    pub caminando: f32, // 0 quieto .. 1 caminando (mezcla la animacion)
}

// el primer objeto "solido" que toca el rayo. se saltean las particulas
// (no proyectan sombra) y el agua o el hielo (transparentes que desvian
// la luz); el papel shoji tiene ior 1 y si cuenta como pared
fn primer_solido<'a>(bvh: &'a Bvh, r: &Rayo, t_max: f32) -> Option<Impacto<'a>> {
    let mut desde = 0.0;
    for _ in 0..8 {
        let rr = Rayo::nuevo(r.en(desde), r.dir);
        let imp = bvh.intersectar(&rr, t_max - desde)?;
        let m = &imp.objeto.material;
        let ignorar = !imp.objeto.proyecta_sombra || (m.transparencia > 0.0 && m.ior > 1.0);
        if !ignorar {
            return Some(imp);
        }
        desde += imp.t + EPS;
    }
    None
}

// altura del piso debajo de (x, z), o None si ahi no se puede pararse
// (afuera de la isla, agua, una superficie muy inclinada...)
fn suelo(escena: &Escena, x: f32, z: f32, y_pies: f32) -> Option<f32> {
    let r = Rayo::nuevo(v3(x, y_pies + ESCALON + 0.05, z), v3(0.0, -1.0, 0.0));
    let imp = primer_solido(&escena.estatica, &r, 4.0)?;
    // si el rayo arranco adentro de algo, sale por una cara de abajo
    // (normal hacia abajo) y eso no cuenta como piso. no pedimos que la
    // superficie sea horizontal: en las juntas del puente el rayo puede
    // pegar en la punta de un tablon inclinado, y las paredes ya las
    // frenan los rayos de choque y el limite del escalon
    if imp.normal_geo.y < 0.1 || imp.punto.y < Y_MIN {
        return None;
    }
    Some(imp.punto.y)
}

// diferencia de angulos llevada a [-pi, pi]
fn diferencia(a: f32, b: f32) -> f32 {
    (a - b + PI).rem_euclid(2.0 * PI) - PI
}

impl Personaje {
    // arranca en el camino de entrada, entre el torii y el puente,
    // mirando hacia el templo
    pub fn nuevo(escena: &Escena) -> Personaje {
        let (x, z) = (0.0, 8.8);
        let y = suelo(escena, x, z, 0.5).unwrap_or(0.03);
        Personaje { pos: v3(x, y, z), rumbo: PI, fase: 0.0, caminando: 0.0 }
    }

    // posicion final si avanza "d" desde donde esta, o None si choca
    fn intentar(&self, escena: &Escena, d: Vec3) -> Option<Vec3> {
        let largo = d.largo();
        if largo < 1e-6 {
            return None;
        }
        let u = d / largo;
        // tres rayos en abanico por altura: el del medio y dos a los
        // costados, para que el cuerpo (no solo su centro) no se meta
        for h in ALTURAS_CHOQUE {
            let o = self.pos + v3(0.0, h, 0.0);
            for ang in [-0.6f32, 0.0, 0.6] {
                let r = Rayo::nuevo(o, Mat3::rot_y(ang).por(u));
                let alcance = RADIO + largo;
                if primer_solido(&escena.estatica, &r, alcance).is_some() || primer_solido(&escena.dinamica, &r, alcance).is_some() {
                    return None;
                }
            }
        }
        let nueva = self.pos + d;
        let y = suelo(escena, nueva.x, nueva.z, self.pos.y)?;
        Some(v3(nueva.x, y, nueva.z))
    }

    // "dir" es la direccion pedida en el plano del piso (largo 0..1).
    // devuelve true si cambio algo que hay que volver a renderizar
    pub fn mover(&mut self, escena: &Escena, dir: Vec3, dt: f32) -> bool {
        let dt = dt.min(0.1);
        let antes = *self;

        if dir.largo() > 1e-3 {
            // gira de a poco hacia donde camina
            let objetivo = dir.x.atan2(dir.z);
            self.rumbo += diferencia(objetivo, self.rumbo) * (12.0 * dt).min(1.0);

            // si choca de frente, intenta deslizarse a lo largo de la pared
            let d = dir * (VELOCIDAD * dt);
            let destino = self
                .intentar(escena, d)
                .or_else(|| self.intentar(escena, v3(d.x, 0.0, 0.0)))
                .or_else(|| self.intentar(escena, v3(0.0, 0.0, d.z)));
            if let Some(p) = destino {
                self.pos = p;
                self.fase += dt * 11.0;
                self.caminando = (self.caminando + dt * 6.0).min(1.0);
            } else {
                self.caminando = (self.caminando - dt * 6.0).max(0.0);
            }
        } else if self.caminando > 0.0 {
            // al soltar las flechas las piernas vuelven a la posicion quieta
            self.caminando = (self.caminando - dt * 5.0).max(0.0);
        }
        *self != antes
    }

    // centro aproximado del cuerpo (para que la camara lo siga)
    pub fn centro(&self) -> Vec3 {
        self.pos + v3(0.0, 0.5, 0.0)
    }

    // arma el modelo: se construye mirando hacia +Z con los pies en el
    // origen y despues se gira segun el rumbo y se lleva a su posicion
    pub fn crear_objetos(&self, v: &mut Vec<Objeto>) {
        let balanceo = self.fase.sin() * 0.55 * self.caminando;
        let rebote = self.fase.cos().abs() * 0.025 * self.caminando;
        let p = self.pos;
        let rot = Mat3::rot_y(self.rumbo);
        let en = |x: f32, y: f32, z: f32| p + v3(x, y + rebote, z);
        let mut partes = Vec::new();

        // piernas y sandalias: giran alrededor de la cadera
        for (sg, a) in [(-1.0f32, balanceo), (1.0, -balanceo)] {
            let cadera = en(sg * 0.06, 0.34, 0.0);
            let pie = cadera + v3(0.0, -0.31 * a.cos(), 0.31 * a.sin());
            partes.push(barra(cadera, pie, 0.035, TELA_OSCURA));
            partes.push(caja(pie + v3(0.0, -0.005, 0.03), v3(0.035, 0.02, 0.06), PAJA));
        }

        // kimono, obi y brazos (se balancean al reves que las piernas)
        partes.push(cono(en(0.0, 0.28, 0.0), 0.15, 0.11, 0.32, TELA_KIMONO));
        partes.push(cilindro(en(0.0, 0.41, 0.0), 0.123, 0.07, TELA_OBI));
        for (sg, a) in [(-1.0f32, -balanceo), (1.0, balanceo)] {
            let hombro = en(sg * 0.13, 0.56, 0.0);
            let mano = hombro + v3(sg * 0.03, -0.23 * a.cos(), 0.23 * a.sin() * 0.8);
            partes.push(barra(hombro, mano, 0.033, TELA_KIMONO));
            partes.push(esfera(mano, 0.036, PIEL));
        }

        // cabeza, pelo, ojos y sombrero de paja (kasa)
        partes.push(esfera(en(0.0, 0.7, 0.0), 0.1, PIEL));
        partes.push(esfera(en(0.0, 0.72, -0.025), 0.095, CABELLO));
        for sg in [-1.0f32, 1.0] {
            partes.push(esfera(en(sg * 0.035, 0.71, 0.093), 0.013, CABELLO));
        }
        partes.push(cono(en(0.0, 0.76, 0.0), 0.25, 0.02, 0.13, PAJA));

        v.extend(partes.into_iter().map(|o| o.girado_en(p, rot)));
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::interaccion::EstadoDiorama;

    fn caminar(p: &mut Personaje, escena: &Escena, dir: Vec3, segundos: f32) {
        for _ in 0..(segundos / 0.05) as usize {
            p.mover(escena, dir, 0.05);
        }
    }

    #[test]
    fn sube_el_puente_y_la_escalera_del_templo() {
        let estado = EstadoDiorama::default();
        let escena = Escena::nueva(&estado);
        let mut p = Personaje::nuevo(&escena);
        let mut y_max = 0.0f32;
        for _ in 0..300 {
            p.mover(&escena, v3(0.0, 0.0, -1.0), 0.05);
            if p.pos.z > 1.0 && p.pos.z < 6.6 {
                y_max = y_max.max(p.pos.y);
            }
        }
        // paso por arriba del puente
        assert!(y_max > 0.9, "altura en el puente: {y_max}");
        // subio la escalera hasta la veranda y lo freno la caja de
        // ofrendas que esta frente a la puerta
        assert!((p.pos.y - crate::escena::templo::Y_PISO).abs() < 0.05, "pos = {:?}", p.pos);
        assert!(p.pos.z > -4.4 && p.pos.z < -3.9, "pos = {:?}", p.pos);
    }

    #[test]
    fn no_se_mete_al_estanque() {
        let estado = EstadoDiorama::default();
        let escena = Escena::nueva(&estado);
        let mut p = Personaje::nuevo(&escena);
        // hasta el medio del puente y despues hacia el costado
        caminar(&mut p, &escena, v3(0.0, 0.0, -1.0), (8.8 - 3.8) / VELOCIDAD);
        caminar(&mut p, &escena, v3(1.0, 0.0, 0.0), 3.0);
        assert!(p.pos.x < 1.0 && p.pos.y > 0.5, "pos = {:?}", p.pos);
    }
}
