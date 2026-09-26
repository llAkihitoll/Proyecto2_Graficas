// texturas: se cargan desde PNG con raylib (solo para leer el archivo) y
// se guardan como una tabla de colores. el muestreo es
// nuestro: coordenadas uv que se repiten (wrap) y filtrado bilineal.

use crate::matematica::{v3, Vec3};
use raylib::prelude::*;

// los texeles se guardan como bytes (como en el PNG) y se pasan a
// flotante al muestrear: con 4 skyboxes la diferencia de memoria es grande
pub struct Textura {
    pub ancho: usize,
    pub alto: usize,
    pub datos: Vec<[u8; 3]>,
}

#[inline]
fn a_vec3(c: [u8; 3]) -> Vec3 {
    const K: f32 = 1.0 / 255.0;
    v3(c[0] as f32 * K, c[1] as f32 * K, c[2] as f32 * K)
}

impl Textura {
    pub fn cargar(ruta: &str) -> Result<Textura, String> {
        let img = Image::load_image(ruta).map_err(|e| format!("no se pudo cargar {ruta}: {e}"))?;
        let ancho = img.width() as usize;
        let alto = img.height() as usize;
        let colores = img.get_image_data();
        let datos = colores.iter().map(|c| [c.r, c.g, c.b]).collect();
        Ok(Textura { ancho, alto, datos })
    }

    #[inline]
    fn texel(&self, x: i32, y: i32) -> Vec3 {
        let x = x.rem_euclid(self.ancho as i32) as usize;
        let y = y.rem_euclid(self.alto as i32) as usize;
        a_vec3(self.datos[y * self.ancho + x])
    }

    // uv en "vueltas de textura": 1.0 = un mosaico entero, y se repite
    pub fn muestrear(&self, u: f32, v: f32) -> Vec3 {
        let x = u * self.ancho as f32 - 0.5;
        let y = v * self.alto as f32 - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let (xi, yi) = (x0 as i32, y0 as i32);
        let a = self.texel(xi, yi).lerp(self.texel(xi + 1, yi), fx);
        let b = self.texel(xi, yi + 1).lerp(self.texel(xi + 1, yi + 1), fx);
        a.lerp(b, fy)
    }

    // igual que muestrear, pero sin repetir: en el borde se queda con el
    // ultimo pixel (sirve para las caras del skybox, que no se repiten)
    pub fn muestrear_borde(&self, u: f32, v: f32) -> Vec3 {
        let x = (u * self.ancho as f32 - 0.5).clamp(0.0, (self.ancho - 1) as f32);
        let y = (v * self.alto as f32 - 0.5).clamp(0.0, (self.alto - 1) as f32);
        let x0 = (x.floor() as usize).min(self.ancho - 2);
        let y0 = (y.floor() as usize).min(self.alto - 2);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let p = |xx: usize, yy: usize| a_vec3(self.datos[yy * self.ancho + xx]);
        let a = p(x0, y0).lerp(p(x0 + 1, y0), fx);
        let b = p(x0, y0 + 1).lerp(p(x0 + 1, y0 + 1), fx);
        a.lerp(b, fy)
    }
}

// indices de cada textura dentro de Texturas::lista. el orden tiene que
// coincidir con ARCHIVOS
pub const TEX_MADERA: usize = 0;
pub const TEX_PIEDRA: usize = 1;
pub const TEX_METAL: usize = 2;
pub const TEX_AGUA: usize = 3;
pub const TEX_VIDRIO: usize = 4;
pub const TEX_TEJAS: usize = 5;
pub const TEX_PAPEL: usize = 6;
pub const TEX_FOLLAJE: usize = 7;
pub const TEX_PASTO: usize = 8;
pub const TEX_GRAVA: usize = 9;
pub const TEX_CORTEZA: usize = 10;
pub const TEX_NIEVE: usize = 11;
pub const TEX_HIELO: usize = 12;

pub const ARCHIVOS: [&str; 13] = [
    "wood.png",
    "stone.png",
    "metal.png",
    "water.png",
    "glass.png",
    "roof_tiles.png",
    "paper.png",
    "leaves.png",
    "grass.png",
    "gravel.png",
    "bark.png",
    "snow.png",
    "ice.png",
];

pub const CARPETA_TEXTURAS: &str = "assets/textures";

pub struct Texturas {
    pub lista: Vec<Textura>,
}

impl Texturas {
    pub fn cargar_todas() -> Result<Texturas, String> {
        let mut lista = Vec::new();
        for nombre in ARCHIVOS {
            lista.push(Textura::cargar(&format!("{CARPETA_TEXTURAS}/{nombre}"))?);
        }
        Ok(Texturas { lista })
    }
}
