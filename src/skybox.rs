// skybox: un cubo enorme alrededor de la escena con una imagen en cada
// cara. cuando un rayo no choca con nada, en vez de pintar un color fijo
// miramos hacia que cara del cubo apunta su direccion y leemos esa imagen.
//
// hay un skybox por momento del dia (amanecer, dia, atardecer, noche) y
// el render mezcla dos segun la hora (ver ambiente.rs). el sol y la luna
// NO estan pintados en las imagenes: se dibujan al trazar, en su
// posicion real, para que se muevan con el ciclo.

use crate::matematica::{v3, Vec3};
use crate::textura::Textura;

pub const CARPETA_SKYBOX: &str = "assets/skybox";

// una carpeta por momento, en el orden de ambiente::AMANECER..NOCHE
pub const MOMENTOS: [&str; 4] = ["amanecer", "dia", "atardecer", "noche"];

// orden de las caras: +X, -X, +Y, -Y, +Z, -Z
pub const CARAS: [&str; 6] = ["px.png", "nx.png", "py.png", "ny.png", "pz.png", "nz.png"];

// (cara, u, v) con u, v en [-1, 1]  ->  direccion del mundo
pub fn direccion_de_cara(cara: usize, u: f32, v: f32) -> Vec3 {
    match cara {
        0 => v3(1.0, -v, -u),
        1 => v3(-1.0, -v, u),
        2 => v3(u, 1.0, v),
        3 => v3(u, -1.0, -v),
        4 => v3(u, -v, 1.0),
        _ => v3(-u, -v, -1.0),
    }
}

// direccion -> (cara, u, v). es la inversa exacta de direccion_de_cara:
// la cara es el eje donde la direccion tiene la componente mas grande
pub fn cara_de_direccion(d: Vec3) -> (usize, f32, f32) {
    let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
    if ax >= ay && ax >= az {
        if d.x > 0.0 {
            (0, -d.z / ax, -d.y / ax)
        } else {
            (1, d.z / ax, -d.y / ax)
        }
    } else if ay >= az {
        if d.y > 0.0 {
            (2, d.x / ay, d.z / ay)
        } else {
            (3, d.x / ay, -d.z / ay)
        }
    } else if d.z > 0.0 {
        (4, d.x / az, -d.y / az)
    } else {
        (5, -d.x / az, -d.y / az)
    }
}

pub struct Skybox {
    caras: Vec<Textura>,
}

impl Skybox {
    pub fn cargar(momento: &str) -> Result<Skybox, String> {
        let mut caras = Vec::new();
        for nombre in CARAS {
            caras.push(Textura::cargar(&format!("{CARPETA_SKYBOX}/{momento}/{nombre}"))?);
        }
        Ok(Skybox { caras })
    }

    // los 4 skyboxes del ciclo, en el orden de MOMENTOS
    pub fn cargar_todos() -> Result<Vec<Skybox>, String> {
        MOMENTOS.iter().map(|m| Skybox::cargar(m)).collect()
    }

    pub fn muestrear(&self, d: Vec3) -> Vec3 {
        let (cara, u, v) = cara_de_direccion(d);
        // de [-1, 1] a [0, 1]
        self.caras[cara].muestrear_borde(u * 0.5 + 0.5, v * 0.5 + 0.5)
    }
}
