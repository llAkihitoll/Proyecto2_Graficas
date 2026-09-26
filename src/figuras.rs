// figuras primitivas y su interseccion con un rayo.
//
// cada objeto (salvo el triangulo) vive en su propio espacio "local":
// centrado en el origen, sin rotar y sin escalar. para probar si un rayo
// le pega, llevamos el rayo a ese espacio (restar la posicion, aplicar la
// rotacion inversa y dividir por la escala) y ahi resolvemos la
// interseccion con la formula simple de cada figura. el parametro t que
// sale sirve tal cual en el mundo, porque la transformacion es lineal.

use crate::material::Material;
use crate::matematica::{v3, Mat3, Vec3};

pub const EPS: f32 = 1e-3;

#[derive(Clone, Copy, Debug)]
pub struct Rayo {
    pub origen: Vec3,
    pub dir: Vec3,
}

impl Rayo {
    pub fn nuevo(origen: Vec3, dir: Vec3) -> Rayo {
        Rayo { origen, dir }
    }

    #[inline]
    pub fn en(&self, t: f32) -> Vec3 {
        self.origen + self.dir * t
    }
}

// caja alineada a los ejes: sirve para descartar rapido (BVH)
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const VACIA: Aabb = Aabb { min: v3(f32::MAX, f32::MAX, f32::MAX), max: v3(f32::MIN, f32::MIN, f32::MIN) };

    pub fn unir(&self, o: &Aabb) -> Aabb {
        Aabb { min: self.min.min(o.min), max: self.max.max(o.max) }
    }

    pub fn agregar(&self, p: Vec3) -> Aabb {
        Aabb { min: self.min.min(p), max: self.max.max(p) }
    }

    pub fn centro(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn contiene(&self, p: Vec3, margen: f32) -> bool {
        p.x >= self.min.x - margen
            && p.x <= self.max.x + margen
            && p.y >= self.min.y - margen
            && p.y <= self.max.y + margen
            && p.z >= self.min.z - margen
            && p.z <= self.max.z + margen
    }

    // slab test con la inversa de la direccion ya calculada
    #[inline]
    pub fn choca(&self, o: Vec3, inv: Vec3, t_max: f32) -> Option<f32> {
        let tx1 = (self.min.x - o.x) * inv.x;
        let tx2 = (self.max.x - o.x) * inv.x;
        let ty1 = (self.min.y - o.y) * inv.y;
        let ty2 = (self.max.y - o.y) * inv.y;
        let tz1 = (self.min.z - o.z) * inv.z;
        let tz2 = (self.max.z - o.z) * inv.z;
        let t_in = tx1.min(tx2).max(ty1.min(ty2)).max(tz1.min(tz2));
        let t_out = tx1.max(tx2).min(ty1.max(ty2)).min(tz1.max(tz2));
        if t_out >= t_in.max(0.0) && t_in < t_max {
            Some(t_in)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Triangulo {
    pub v0: Vec3,
    pub e1: Vec3,
    pub e2: Vec3,
    pub n: [Vec3; 3],
    pub uv: [[f32; 2]; 3],
}

#[derive(Clone, Copy, Debug)]
pub enum Forma {
    // caja de lados 2*mitad
    Caja { mitad: Vec3 },
    // esfera de radio 1 (la escala la convierte en elipsoide)
    Esfera,
    // cilindro o tronco de cono a lo largo de Y, de -mitad_alto a +mitad_alto
    Cilindro { r_abajo: f32, r_arriba: f32, mitad_alto: f32 },
    // triangulo en coordenadas del mundo (para el techo curvo)
    Triangulo(Triangulo),
}

// perturbaciones de la normal: dan relieve sin agregar geometria
#[derive(Clone, Copy, Debug)]
pub enum Relieve {
    Ninguno,
    // columnas de tejas siguiendo la coordenada u de la textura
    Tejas,
    // ondas en el agua. radial = anillos alrededor de (cx, cz)
    Ondas { cx: f32, cz: f32, amplitud: f32, tiempo: f32, radial: bool },
}

// objetos con los que se puede interactuar (para el click / tecla E)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interactivo {
    Ninguno,
    Campana,
    Linterna,
    Fuente,
    Puerta,
}

#[derive(Clone, Copy, Debug)]
pub struct Objeto {
    pub forma: Forma,
    pub centro: Vec3,
    pub rot: Mat3,   // local -> mundo
    pub rot_t: Mat3, // mundo -> local
    pub escala: Vec3,
    pub inv_escala: Vec3,
    pub material: Material,
    // material de la cara de atras (la parte de abajo del techo es madera)
    pub reverso: Option<Material>,
    pub relieve: Relieve,
    pub id: Interactivo,
    pub proyecta_sombra: bool,
}

pub struct Impacto<'a> {
    pub t: f32,
    pub punto: Vec3,
    pub normal: Vec3,     // normal para iluminar (suavizada), hacia afuera
    pub normal_geo: Vec3, // normal real de la superficie, hacia afuera
    pub uv: (f32, f32),   // en metros; el material la multiplica por escala_uv
    pub objeto: &'a Objeto,
}

impl Objeto {
    pub fn nuevo(forma: Forma, centro: Vec3, material: Material) -> Objeto {
        Objeto {
            forma,
            centro,
            rot: Mat3::IDENTIDAD,
            rot_t: Mat3::IDENTIDAD,
            escala: Vec3::UNO,
            inv_escala: Vec3::UNO,
            material,
            reverso: None,
            relieve: Relieve::Ninguno,
            id: Interactivo::Ninguno,
            proyecta_sombra: true,
        }
    }

    // --- modificadores encadenables ---

    pub fn rotado(mut self, r: Mat3) -> Objeto {
        self.rot = r.tras(&self.rot);
        self.rot_t = self.rot.transpuesta();
        self
    }

    pub fn escalado(mut self, e: Vec3) -> Objeto {
        self.escala = e;
        self.inv_escala = v3(1.0 / e.x, 1.0 / e.y, 1.0 / e.z);
        self
    }

    // gira el objeto entero (posicion y orientacion) alrededor de un pivote
    pub fn girado_en(mut self, pivote: Vec3, r: Mat3) -> Objeto {
        self.centro = pivote + r.por(self.centro - pivote);
        self.rotado(r)
    }

    pub fn con_id(mut self, id: Interactivo) -> Objeto {
        self.id = id;
        self
    }

    pub fn con_reverso(mut self, m: Material) -> Objeto {
        self.reverso = Some(m);
        self
    }

    pub fn con_relieve(mut self, r: Relieve) -> Objeto {
        self.relieve = r;
        self
    }

    pub fn sin_sombra(mut self) -> Objeto {
        self.proyecta_sombra = false;
        self
    }

    pub fn aabb(&self) -> Aabb {
        let local = match self.forma {
            Forma::Caja { mitad } => mitad,
            Forma::Esfera => Vec3::UNO,
            Forma::Cilindro { r_abajo, r_arriba, mitad_alto } => {
                let r = r_abajo.max(r_arriba);
                v3(r, mitad_alto, r)
            }
            Forma::Triangulo(t) => {
                let a = t.v0;
                let b = t.v0 + t.e1;
                let c = t.v0 + t.e2;
                let e = v3(1e-3, 1e-3, 1e-3);
                return Aabb { min: a.min(b).min(c) - e, max: a.max(b).max(c) + e };
            }
        };
        let mut caja = Aabb::VACIA;
        for i in 0..8 {
            let esquina = v3(
                if i & 1 == 0 { -local.x } else { local.x },
                if i & 2 == 0 { -local.y } else { local.y },
                if i & 4 == 0 { -local.z } else { local.z },
            );
            caja = caja.agregar(self.centro + self.rot.por(esquina.mul(self.escala)));
        }
        caja
    }

    pub fn intersectar(&self, r: &Rayo, t_max: f32) -> Option<Impacto<'_>> {
        if let Forma::Triangulo(tri) = &self.forma {
            return self.intersectar_triangulo(tri, r, t_max);
        }

        // rayo en espacio local
        let o = self.rot_t.por(r.origen - self.centro).mul(self.inv_escala);
        let d = self.rot_t.por(r.dir).mul(self.inv_escala);

        let (t, n_local) = match self.forma {
            Forma::Caja { mitad } => caja_local(o, d, mitad, t_max)?,
            Forma::Esfera => esfera_local(o, d, t_max)?,
            Forma::Cilindro { r_abajo, r_arriba, mitad_alto } => cilindro_local(o, d, r_abajo, r_arriba, mitad_alto, t_max)?,
            Forma::Triangulo(_) => unreachable!(),
        };

        let p_local = o + d * t;
        // la normal se transforma con la inversa de la escala (si no, en
        // un elipsoide quedaria torcida) y despues se rota al mundo
        let normal = self.rot.por(n_local.mul(self.inv_escala)).normalizado();
        // punto en el espacio del objeto pero en metros (para la textura)
        let p = p_local.mul(self.escala);

        let uv = match self.forma {
            Forma::Cilindro { r_abajo, r_arriba, .. } => {
                if n_local.x == 0.0 && n_local.z == 0.0 {
                    (p.x, p.z) // tapa
                } else {
                    // pared: la textura se enrolla alrededor (angulo * radio)
                    let radio = 0.5 * (r_abajo + r_arriba) * 0.5 * (self.escala.x + self.escala.z);
                    (p.z.atan2(p.x) * radio.max(0.05), p.y)
                }
            }
            _ => {
                // proyeccion segun el eje dominante de la normal local
                let (ax, ay, az) = (n_local.x.abs(), n_local.y.abs(), n_local.z.abs());
                if ax >= ay && ax >= az {
                    (p.z, p.y)
                } else if ay >= az {
                    (p.x, p.z)
                } else {
                    (p.x, p.y)
                }
            }
        };

        Some(Impacto { t, punto: r.en(t), normal, normal_geo: normal, uv, objeto: self })
    }

    // Moller-Trumbore: resuelve origen + t*dir = v0 + u*e1 + v*e2
    fn intersectar_triangulo(&self, tri: &Triangulo, r: &Rayo, t_max: f32) -> Option<Impacto<'_>> {
        let p = r.dir.cross(tri.e2);
        let det = tri.e1.dot(p);
        if det.abs() < 1e-9 {
            return None; // rayo paralelo al triangulo
        }
        let inv = 1.0 / det;
        let s = r.origen - tri.v0;
        let u = s.dot(p) * inv;
        // una tolerancia minima: sin ella, rayos que pasan justo por la
        // arista compartida entre dos triangulos pueden colarse entre ambos
        const TOL: f32 = 1e-5;
        if !(-TOL..=1.0 + TOL).contains(&u) {
            return None;
        }
        let q = s.cross(tri.e1);
        let v = r.dir.dot(q) * inv;
        if v < -TOL || u + v > 1.0 + TOL {
            return None;
        }
        let t = tri.e2.dot(q) * inv;
        if t <= EPS || t >= t_max {
            return None;
        }
        let w = 1.0 - u - v;
        let normal = (tri.n[0] * w + tri.n[1] * u + tri.n[2] * v).normalizado();
        let mut normal_geo = tri.e1.cross(tri.e2).normalizado();
        if normal_geo.dot(normal) < 0.0 {
            normal_geo = -normal_geo;
        }
        let uv = (
            tri.uv[0][0] * w + tri.uv[1][0] * u + tri.uv[2][0] * v,
            tri.uv[0][1] * w + tri.uv[1][1] * u + tri.uv[2][1] * v,
        );
        Some(Impacto { t, punto: r.en(t), normal, normal_geo, uv, objeto: self })
    }
}

// slab test de caja centrada en el origen. si el rayo arranca adentro
// (por ejemplo un rayo refractado dentro del agua) devuelve la salida
fn caja_local(o: Vec3, d: Vec3, mitad: Vec3, t_max: f32) -> Option<(f32, Vec3)> {
    let mut t_in = f32::NEG_INFINITY;
    let mut t_out = f32::INFINITY;
    let mut eje_in = 0;
    let mut eje_out = 0;
    let mut signo_in = 0.0;
    let mut signo_out = 0.0;

    for eje in 0..3 {
        let oe = o.eje(eje);
        let de = d.eje(eje);
        let m = mitad.eje(eje);
        if de.abs() < 1e-12 {
            if oe < -m || oe > m {
                return None;
            }
            continue;
        }
        let inv = 1.0 / de;
        let mut t1 = (-m - oe) * inv;
        let mut t2 = (m - oe) * inv;
        let mut s1 = -1.0;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            s1 = 1.0;
        }
        if t1 > t_in {
            t_in = t1;
            eje_in = eje;
            signo_in = s1;
        }
        if t2 < t_out {
            t_out = t2;
            eje_out = eje;
            signo_out = -s1;
        }
    }

    if t_in > t_out {
        return None;
    }
    let (t, eje, signo) = if t_in > EPS {
        (t_in, eje_in, signo_in)
    } else if t_out > EPS {
        (t_out, eje_out, signo_out)
    } else {
        return None;
    };
    if t >= t_max {
        return None;
    }
    let mut n = Vec3::CERO;
    match eje {
        0 => n.x = signo,
        1 => n.y = signo,
        _ => n.z = signo,
    }
    Some((t, n))
}

// esfera unitaria: |o + t d|^2 = 1
fn esfera_local(o: Vec3, d: Vec3, t_max: f32) -> Option<(f32, Vec3)> {
    let a = d.dot(d);
    let b = o.dot(d);
    let c = o.dot(o) - 1.0;
    let disc = b * b - a * c;
    if disc < 0.0 {
        return None;
    }
    let raiz = disc.sqrt();
    let mut t = (-b - raiz) / a;
    if t <= EPS {
        t = (-b + raiz) / a;
    }
    if t <= EPS || t >= t_max {
        return None;
    }
    Some((t, o + d * t))
}

// tronco de cono a lo largo de Y (si r_abajo = r_arriba es un cilindro).
// el radio cambia linealmente con la altura: r(y) = a + k*y, y la pared
// cumple x^2 + z^2 = r(y)^2. reemplazando el rayo queda una cuadratica.
fn cilindro_local(o: Vec3, d: Vec3, r0: f32, r1: f32, h: f32, t_max: f32) -> Option<(f32, Vec3)> {
    let a = 0.5 * (r0 + r1);
    let k = (r1 - r0) / (2.0 * h);
    let ry = a + k * o.y;

    let qa = d.x * d.x + d.z * d.z - k * k * d.y * d.y;
    let qb = 2.0 * (o.x * d.x + o.z * d.z - k * d.y * ry);
    let qc = o.x * o.x + o.z * o.z - ry * ry;

    let mut mejor: Option<(f32, Vec3)> = None;
    let probar = |t: f32, n: Vec3, mejor: &mut Option<(f32, Vec3)>| {
        if t > EPS && t < t_max && mejor.map_or(true, |(tb, _)| t < tb) {
            *mejor = Some((t, n));
        }
    };

    // pared lateral
    let mut raices = [f32::NAN; 2];
    if qa.abs() > 1e-9 {
        let disc = qb * qb - 4.0 * qa * qc;
        if disc >= 0.0 {
            let s = disc.sqrt();
            raices = [(-qb - s) / (2.0 * qa), (-qb + s) / (2.0 * qa)];
        }
    } else if qb.abs() > 1e-9 {
        raices[0] = -qc / qb;
    }
    for t in raices {
        if t.is_nan() {
            continue;
        }
        let p = o + d * t;
        let r = a + k * p.y;
        if p.y.abs() <= h && r >= 0.0 {
            probar(t, v3(p.x, -k * r, p.z), &mut mejor);
        }
    }

    // tapas de arriba y de abajo
    if d.y.abs() > 1e-9 {
        for (y, r, ny) in [(h, r1, 1.0), (-h, r0, -1.0)] {
            if r <= 1e-4 {
                continue;
            }
            let t = (y - o.y) / d.y;
            let p = o + d * t;
            if p.x * p.x + p.z * p.z <= r * r {
                probar(t, v3(0.0, ny, 0.0), &mut mejor);
            }
        }
    }
    mejor
}

// =====================================================================
// constructores cortos para armar la escena
// =====================================================================

pub fn caja(centro: Vec3, mitad: Vec3, m: Material) -> Objeto {
    Objeto::nuevo(Forma::Caja { mitad }, centro, m)
}

// caja a partir de sus esquinas minima y maxima
pub fn caja_mm(min: Vec3, max: Vec3, m: Material) -> Objeto {
    caja((min + max) * 0.5, (max - min) * 0.5, m)
}

pub fn esfera(centro: Vec3, radio: f32, m: Material) -> Objeto {
    Objeto::nuevo(Forma::Esfera, centro, m).escalado(v3(radio, radio, radio))
}

pub fn elipsoide(centro: Vec3, radios: Vec3, m: Material) -> Objeto {
    Objeto::nuevo(Forma::Esfera, centro, m).escalado(radios)
}

// cilindro vertical apoyado en "base" (y = base.y) de alto "alto"
pub fn cilindro(base: Vec3, radio: f32, alto: f32, m: Material) -> Objeto {
    cono(base, radio, radio, alto, m)
}

// tronco de cono vertical apoyado en "base"
pub fn cono(base: Vec3, r_abajo: f32, r_arriba: f32, alto: f32, m: Material) -> Objeto {
    Objeto::nuevo(
        Forma::Cilindro { r_abajo, r_arriba, mitad_alto: alto * 0.5 },
        base + v3(0.0, alto * 0.5, 0.0),
        m,
    )
}

// caja larga que va del punto a al punto b (vigas inclinadas, barandales
// curvos del puente, cumbreras del techo). "ancho" es de costado y
// "alto" hacia arriba
pub fn viga(a: Vec3, b: Vec3, ancho: f32, alto: f32, m: Material) -> Objeto {
    let eje = b - a;
    let largo = eje.largo();
    let x = eje / largo;
    let referencia = if x.y.abs() > 0.95 { v3(1.0, 0.0, 0.0) } else { Vec3::ARRIBA };
    let z = x.cross(referencia).normalizado();
    let y = z.cross(x);
    Objeto::nuevo(Forma::Caja { mitad: v3(largo * 0.5, alto * 0.5, ancho * 0.5) }, (a + b) * 0.5, m)
        .rotado(Mat3::desde_columnas(x, y, z))
}

// cilindro que va del punto a al punto b (cuerdas, mazo de la campana)
pub fn barra(a: Vec3, b: Vec3, radio: f32, m: Material) -> Objeto {
    let eje = b - a;
    let largo = eje.largo();
    let y = eje / largo;
    let referencia = if y.x.abs() > 0.9 { v3(0.0, 0.0, 1.0) } else { v3(1.0, 0.0, 0.0) };
    let z = referencia.cross(y).normalizado();
    let x = y.cross(z);
    Objeto::nuevo(Forma::Cilindro { r_abajo: radio, r_arriba: radio, mitad_alto: largo * 0.5 }, (a + b) * 0.5, m)
        .rotado(Mat3::desde_columnas(x, y, z))
}

pub fn triangulo(p: [Vec3; 3], n: [Vec3; 3], uv: [[f32; 2]; 3], m: Material) -> Objeto {
    let tri = Triangulo { v0: p[0], e1: p[1] - p[0], e2: p[2] - p[0], n, uv };
    Objeto::nuevo(Forma::Triangulo(tri), p[0], m)
}
