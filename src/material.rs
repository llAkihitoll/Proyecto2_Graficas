// materiales: cada uno define como responde una superficie a la luz y a
// los rayos secundarios.
//   albedo:        tinte que multiplica el color de la textura (el color
//                  "propio" que devuelve la superficie bajo luz difusa)
//   textura:       indice de la textura (ver textura.rs) o None = liso
//   escala_uv:     cuantas veces se repite la textura por metro
//   especular:     intensidad del brillo tipo Blinn-Phong
//   brillo:        exponente del brillo (mas alto = brillo mas chico/puntual)
//   reflectividad: fraccion de luz que se refleja como espejo (rayo reflejado)
//   transparencia: fraccion de luz que atraviesa la superficie (rayo refractado)
//   ior:           indice de refraccion (1.0 = no desvia el rayo)
//   absorcion:     cuanto color se "come" el medio por metro recorrido
//                  adentro (ley de Beer). le da al agua su tono verdoso
//   emision:       luz propia (llamas, objetos encendidos)

use crate::matematica::{v3, Vec3};
use crate::textura::*;

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub albedo: Vec3,
    pub textura: Option<usize>,
    pub escala_uv: f32,
    pub especular: f32,
    pub brillo: f32,
    pub reflectividad: f32,
    pub transparencia: f32,
    pub ior: f32,
    pub absorcion: Vec3,
    pub emision: Vec3,
}

const BASE: Material = Material {
    albedo: Vec3::UNO,
    textura: None,
    escala_uv: 1.0,
    especular: 0.0,
    brillo: 1.0,
    reflectividad: 0.0,
    transparencia: 0.0,
    ior: 1.0,
    absorcion: Vec3::CERO,
    emision: Vec3::CERO,
};

impl Material {
    // copia del material con otra emision (linternas, campana brillando)
    pub fn con_emision(mut self, e: Vec3) -> Material {
        self.emision = e;
        self
    }

    pub fn con_albedo(mut self, a: Vec3) -> Material {
        self.albedo = a;
        self
    }
}

// =====================================================================
// los 5 materiales principales del proyecto
// =====================================================================

// 1. MADERA: mate, algo de brillo suave de barniz, casi nada de reflejo
pub const MADERA: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_MADERA),
    escala_uv: 0.6,
    especular: 0.25,
    brillo: 24.0,
    reflectividad: 0.03,
    ..BASE
};

// 2. PIEDRA: la mas mate de todas, sin reflejo
pub const PIEDRA: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_PIEDRA),
    escala_uv: 0.45,
    especular: 0.05,
    brillo: 8.0,
    reflectividad: 0.02,
    ..BASE
};

// 3. METAL (bronce): brillo fuerte y concentrado, refleja bastante
pub const METAL: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_METAL),
    escala_uv: 1.2,
    especular: 0.9,
    brillo: 90.0,
    reflectividad: 0.45,
    ..BASE
};

// 4. AGUA: muy transparente, refracta con ior 1.33 y absorbe rojo
pub const AGUA: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_AGUA),
    escala_uv: 0.35,
    especular: 0.9,
    brillo: 160.0,
    reflectividad: 0.12,
    transparencia: 0.8,
    ior: 1.33,
    absorcion: v3(1.1, 0.32, 0.26),
    ..BASE
};

// 5. VIDRIO / CRISTAL: el mas transparente, ior 1.5, brillo muy puntual
pub const VIDRIO: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_VIDRIO),
    escala_uv: 1.0,
    especular: 1.0,
    brillo: 220.0,
    reflectividad: 0.06,
    transparencia: 0.88,
    ior: 1.5,
    absorcion: v3(0.08, 0.02, 0.04),
    ..BASE
};

// =====================================================================
// variantes y materiales de apoyo (derivan de los principales o usan
// texturas propias para distinguir partes del templo y del jardin)
// =====================================================================

// madera lacada bermellon (columnas, vigas, torii, barandales)
pub const LACA_ROJA: Material = Material {
    albedo: v3(1.45, 0.40, 0.26),
    especular: 0.55,
    brillo: 60.0,
    reflectividad: 0.05,
    ..MADERA
};

// laca negra (base de columnas, kasagi del torii)
pub const LACA_NEGRA: Material = Material {
    albedo: v3(0.22, 0.2, 0.2),
    especular: 0.7,
    brillo: 70.0,
    reflectividad: 0.08,
    ..MADERA
};

// madera oscura envejecida (vigas, bordes de techo, pilotes)
pub const MADERA_OSCURA: Material = Material { albedo: v3(0.55, 0.47, 0.42), ..MADERA };

// oro: el mismo metal pero con tinte dorado y mas reflejo
pub const ORO: Material = Material {
    albedo: v3(1.55, 1.2, 0.55),
    especular: 1.0,
    brillo: 120.0,
    reflectividad: 0.5,
    ..METAL
};

// tejas del techo: ceramica esmaltada gris azulada
pub const TEJAS: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_TEJAS),
    escala_uv: 0.5,
    especular: 0.35,
    brillo: 30.0,
    reflectividad: 0.06,
    ..BASE
};

// cumbreras (lomos del techo): las mismas tejas mas oscuras y sin patron
pub const CUMBRERA: Material = Material { albedo: v3(0.55, 0.58, 0.62), escala_uv: 1.5, ..TEJAS };

// yeso / papel de las paredes (opaco)
pub const YESO: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_PAPEL),
    escala_uv: 0.8,
    especular: 0.04,
    brillo: 6.0,
    ..BASE
};

// papel de shoji: translucido (deja pasar algo de luz pero no desvia)
pub const PAPEL_SHOJI: Material = Material { transparencia: 0.3, ior: 1.0, absorcion: v3(0.5, 0.5, 0.6), ..YESO };

pub const PASTO: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_PASTO),
    escala_uv: 0.3,
    especular: 0.03,
    brillo: 4.0,
    ..BASE
};

pub const GRAVA: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_GRAVA),
    escala_uv: 0.4,
    especular: 0.05,
    brillo: 6.0,
    ..BASE
};

// lecho del estanque: grava mas oscura y verdosa
pub const LECHO: Material = Material { albedo: v3(0.95, 0.95, 0.8), escala_uv: 0.9, ..GRAVA };

// tierra / roca de los costados del diorama
pub const TIERRA: Material = Material { albedo: v3(0.72, 0.58, 0.46), escala_uv: 0.3, ..PIEDRA };

// roca natural: la textura de piedra muy chica, asi se lee como grano
pub const ROCA: Material = Material { albedo: v3(0.85, 0.82, 0.78), escala_uv: 3.5, especular: 0.1, brillo: 12.0, ..PIEDRA };

pub const CORTEZA: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_CORTEZA),
    escala_uv: 1.2,
    especular: 0.05,
    brillo: 6.0,
    ..BASE
};

const FOLLAJE: Material = Material {
    albedo: v3(1.0, 1.0, 1.0),
    textura: Some(TEX_FOLLAJE),
    escala_uv: 0.8,
    especular: 0.08,
    brillo: 10.0,
    ..BASE
};

pub const HOJAS_PINO: Material = Material { albedo: v3(0.3, 0.55, 0.36), ..FOLLAJE };
pub const HOJAS_ARCE: Material = Material { albedo: v3(1.15, 0.3, 0.12), ..FOLLAJE };
pub const HOJAS_SAKURA: Material = Material { albedo: v3(1.2, 0.8, 0.92), ..FOLLAJE };
pub const HOJAS_ARBUSTO: Material = Material { albedo: v3(0.45, 0.72, 0.36), ..FOLLAJE };
pub const AZALEA: Material = Material { albedo: v3(1.15, 0.42, 0.72), ..FOLLAJE };
pub const HOJA_LOTO: Material = Material { albedo: v3(0.4, 0.75, 0.32), especular: 0.4, brillo: 40.0, ..FOLLAJE };

pub const BAMBU: Material = Material {
    albedo: v3(0.85, 1.35, 0.6),
    especular: 0.4,
    brillo: 40.0,
    ..MADERA
};

pub const KOI_NARANJA: Material = Material {
    albedo: v3(0.95, 0.42, 0.1),
    especular: 0.6,
    brillo: 50.0,
    ..BASE
};

pub const KOI_BLANCO: Material = Material { albedo: v3(0.92, 0.9, 0.86), ..KOI_NARANJA };

pub const FLOR_LOTO: Material = Material { albedo: v3(0.98, 0.6, 0.75), especular: 0.2, brillo: 20.0, ..BASE };

// llama de linterna: apagada es una mecha oscura; encendida emite luz
pub const MECHA: Material = Material { albedo: v3(0.12, 0.1, 0.09), ..BASE };
pub const LLAMA: Material = Material {
    albedo: v3(1.0, 0.8, 0.5),
    emision: v3(3.2, 1.9, 0.7),
    ..BASE
};

// velas del altar
pub const CERA: Material = Material { albedo: v3(0.95, 0.92, 0.85), especular: 0.2, brillo: 20.0, ..BASE };

// cuerda
pub const CUERDA: Material = Material { albedo: v3(0.75, 0.62, 0.4), ..BASE };
