// camara orbital con perspectiva: mira siempre a un punto "objetivo" y
// se ubica sobre una esfera alrededor de el.
//   theta: angulo horizontal (orbitar alrededor del diorama)
//   phi:   angulo vertical (0 = a la altura del objetivo, pi/2 = cenital)
//   radio: distancia al objetivo (zoom)

use crate::figuras::Rayo;
use crate::matematica::{v3, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camara {
    pub objetivo: Vec3,
    pub theta: f32,
    pub phi: f32,
    pub radio: f32,
    pub fov: f32, // campo de vision vertical en radianes
}

pub const RADIO_MIN: f32 = 4.0;
pub const RADIO_MAX: f32 = 48.0;
pub const PHI_MIN: f32 = 0.03;
pub const PHI_MAX: f32 = 1.45;
// el objetivo no puede salir del diorama
pub const LIMITE_XZ: f32 = 11.0;

// base ortonormal ya calculada para generar los rayos de un frame
pub struct Vista {
    pub posicion: Vec3,
    adelante: Vec3,
    derecha: Vec3,
    arriba: Vec3,
    tan_mitad: f32,
}

impl Camara {
    pub fn vista_general() -> Camara {
        Camara { objetivo: v3(0.0, 2.0, -1.5), theta: 0.55, phi: 0.42, radio: 30.0, fov: 50f32.to_radians() }
    }

    // vistas predefinidas (teclas 1-5) para mostrar partes de la escena
    pub fn preset(n: u32) -> Camara {
        let base = Camara::vista_general();
        match n {
            // templo de frente, desde el camino
            2 => Camara { objetivo: v3(0.0, 4.0, -6.5), theta: 0.05, phi: 0.2, radio: 15.5, ..base },
            // estanque visto desde el patio, contra el atardecer: se ven
            // los reflejos del cielo y, a traves del agua, los koi
            3 => Camara { objetivo: v3(-2.4, 0.0, 3.8), theta: 2.55, phi: 0.3, radio: 8.0, ..base },
            // techo de cerca, desde arriba y en diagonal
            4 => Camara { objetivo: v3(0.0, 6.5, -7.0), theta: -0.75, phi: 0.5, radio: 12.5, ..base },
            // campana y fuente
            5 => Camara { objetivo: v3(0.0, 1.5, -1.2), theta: 0.0, phi: 0.3, radio: 14.0, ..base },
            _ => base,
        }
    }

    pub fn posicion(&self) -> Vec3 {
        let (sp, cp) = self.phi.sin_cos();
        let (st, ct) = self.theta.sin_cos();
        self.objetivo + v3(cp * st, sp, cp * ct) * self.radio
    }

    pub fn limitar(&mut self) {
        self.radio = self.radio.clamp(RADIO_MIN, RADIO_MAX);
        self.phi = self.phi.clamp(PHI_MIN, PHI_MAX);
        self.objetivo.x = self.objetivo.x.clamp(-LIMITE_XZ, LIMITE_XZ);
        self.objetivo.z = self.objetivo.z.clamp(-LIMITE_XZ, LIMITE_XZ);
        self.objetivo.y = self.objetivo.y.clamp(0.0, 9.0);
    }

    // mueve el objetivo en el plano del piso, relativo a hacia donde mira
    pub fn desplazar(&mut self, adelante: f32, derecha: f32) {
        let (frente, lado) = self.ejes_piso();
        self.objetivo += frente * adelante + lado * derecha;
    }

    // direcciones "hacia adelante" y "hacia la derecha" de la camara,
    // aplastadas sobre el piso (para mover al personaje con las flechas)
    pub fn ejes_piso(&self) -> (Vec3, Vec3) {
        let (st, ct) = self.theta.sin_cos();
        (v3(-st, 0.0, -ct), v3(ct, 0.0, -st))
    }

    pub fn vista(&self) -> Vista {
        let posicion = self.posicion();
        let adelante = (self.objetivo - posicion).normalizado();
        let derecha = adelante.cross(Vec3::ARRIBA).normalizado();
        let arriba = derecha.cross(adelante);
        Vista { posicion, adelante, derecha, arriba, tan_mitad: (self.fov * 0.5).tan() }
    }
}

impl Vista {
    // rayo que pasa por el pixel (px, py) de una imagen ancho x alto
    #[inline]
    pub fn rayo(&self, px: f32, py: f32, ancho: f32, alto: f32) -> Rayo {
        let aspecto = ancho / alto;
        let x = (2.0 * px / ancho - 1.0) * aspecto * self.tan_mitad;
        let y = (1.0 - 2.0 * py / alto) * self.tan_mitad;
        let dir = (self.adelante + self.derecha * x + self.arriba * y).normalizado();
        Rayo::nuevo(self.posicion, dir)
    }
}
