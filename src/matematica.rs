// matematica basica del raytracer: vectores 3D y matrices de rotacion.
// usamos un tipo propio (en vez del Vector3 de raylib) porque el mismo
// Vec3 nos sirve tanto para puntos/direcciones como para colores RGB en
// flotante, y asi todas las cuentas del trazado quedan escritas a mano.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[inline]
pub const fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub const CERO: Vec3 = v3(0.0, 0.0, 0.0);
    pub const UNO: Vec3 = v3(1.0, 1.0, 1.0);
    pub const ARRIBA: Vec3 = v3(0.0, 1.0, 0.0);

    #[inline]
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    #[inline]
    pub fn largo(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn normalizado(self) -> Vec3 {
        let l = self.largo();
        if l > 1e-12 {
            self * (1.0 / l)
        } else {
            self
        }
    }

    // producto componente a componente (sirve para "filtrar" un color por otro)
    #[inline]
    pub fn mul(self, o: Vec3) -> Vec3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }

    #[inline]
    pub fn min(self, o: Vec3) -> Vec3 {
        v3(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }

    #[inline]
    pub fn max(self, o: Vec3) -> Vec3 {
        v3(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }

    #[inline]
    pub fn eje(self, i: usize) -> f32 {
        match i {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }

    #[inline]
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self * (1.0 - t) + o * t
    }

    #[inline]
    pub fn luminancia(self) -> f32 {
        0.2126 * self.x + 0.7152 * self.y + 0.0722 * self.z
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, o: Vec3) {
        self.x += o.x;
        self.y += o.y;
        self.z += o.z;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, s: f32) -> Vec3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, s: f32) {
        self.x *= s;
        self.y *= s;
        self.z *= s;
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, s: f32) -> Vec3 {
        self * (1.0 / s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        v3(-self.x, -self.y, -self.z)
    }
}

// reflexion de una direccion d contra una superficie de normal n:
//   r = d - 2 (d.n) n
#[inline]
pub fn reflejar(d: Vec3, n: Vec3) -> Vec3 {
    d - n * (2.0 * d.dot(n))
}

// refraccion por la ley de Snell. "eta" es n1/n2 (indice del medio de
// donde viene el rayo sobre el del medio al que entra). n tiene que
// apuntar hacia el lado de donde viene el rayo. si el angulo es muy
// rasante hay reflexion interna total y no existe rayo refractado.
#[inline]
pub fn refractar(d: Vec3, n: Vec3, eta: f32) -> Option<Vec3> {
    let cos_i = -d.dot(n);
    let sen2_t = eta * eta * (1.0 - cos_i * cos_i);
    if sen2_t > 1.0 {
        return None; // reflexion interna total
    }
    let cos_t = (1.0 - sen2_t).sqrt();
    Some(d * eta + n * (eta * cos_i - cos_t))
}

// aproximacion de Schlick del termino de Fresnel: que fraccion de la luz
// se refleja (el resto se refracta) segun el angulo de incidencia
#[inline]
pub fn schlick(cos_i: f32, n1: f32, n2: f32) -> f32 {
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos_i).clamp(0.0, 1.0).powi(5)
}

// matriz 3x3 (por filas). solo la usamos para rotaciones, asi que su
// inversa es simplemente su transpuesta.
#[derive(Clone, Copy, Debug)]
pub struct Mat3 {
    pub f: [Vec3; 3],
}

impl Mat3 {
    pub const IDENTIDAD: Mat3 = Mat3 { f: [v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), v3(0.0, 0.0, 1.0)] };

    // rotacion alrededor de X (inclina hacia adelante/atras)
    pub fn rot_x(a: f32) -> Mat3 {
        let (s, c) = a.sin_cos();
        Mat3 { f: [v3(1.0, 0.0, 0.0), v3(0.0, c, -s), v3(0.0, s, c)] }
    }

    // rotacion alrededor de Y (gira horizontalmente)
    pub fn rot_y(a: f32) -> Mat3 {
        let (s, c) = a.sin_cos();
        Mat3 { f: [v3(c, 0.0, s), v3(0.0, 1.0, 0.0), v3(-s, 0.0, c)] }
    }

    // rotacion alrededor de Z (inclina hacia los costados)
    pub fn rot_z(a: f32) -> Mat3 {
        let (s, c) = a.sin_cos();
        Mat3 { f: [v3(c, -s, 0.0), v3(s, c, 0.0), v3(0.0, 0.0, 1.0)] }
    }

    // matriz cuyas columnas son las imagenes de los ejes X, Y, Z locales
    pub fn desde_columnas(x: Vec3, y: Vec3, z: Vec3) -> Mat3 {
        Mat3 { f: [v3(x.x, y.x, z.x), v3(x.y, y.y, z.y), v3(x.z, y.z, z.z)] }
    }

    #[inline]
    pub fn por(&self, v: Vec3) -> Vec3 {
        v3(self.f[0].dot(v), self.f[1].dot(v), self.f[2].dot(v))
    }

    pub fn transpuesta(&self) -> Mat3 {
        let [a, b, c] = self.f;
        Mat3 { f: [v3(a.x, b.x, c.x), v3(a.y, b.y, c.y), v3(a.z, b.z, c.z)] }
    }

    // composicion: primero aplica "otra", despues esta
    pub fn tras(&self, otra: &Mat3) -> Mat3 {
        let t = otra.transpuesta();
        Mat3 {
            f: [
                v3(self.f[0].dot(t.f[0]), self.f[0].dot(t.f[1]), self.f[0].dot(t.f[2])),
                v3(self.f[1].dot(t.f[0]), self.f[1].dot(t.f[1]), self.f[1].dot(t.f[2])),
                v3(self.f[2].dot(t.f[0]), self.f[2].dot(t.f[1]), self.f[2].dot(t.f[2])),
            ],
        }
    }
}

#[inline]
pub fn suavizar(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn refraccion_de_frente_no_desvia() {
        let d = v3(0.0, -1.0, 0.0);
        let t = refractar(d, Vec3::ARRIBA, 1.0 / 1.33).unwrap();
        assert!((t - d).largo() < 1e-5);
    }

    #[test]
    fn snell_se_cumple() {
        // sen(i) * n1 = sen(t) * n2
        let d = v3(0.6, -0.8, 0.0);
        let t = refractar(d, Vec3::ARRIBA, 1.0 / 1.5).unwrap();
        assert!((0.6 * 1.0 - t.x.abs() * 1.5).abs() < 1e-4);
    }

    #[test]
    fn reflexion_interna_total() {
        // saliendo del vidrio con angulo rasante no hay rayo refractado
        let d = v3(0.9, 0.435_89, 0.0);
        assert!(refractar(d, v3(0.0, -1.0, 0.0), 1.5).is_none());
    }

    #[test]
    fn rotacion_y_su_inversa() {
        let r = Mat3::rot_y(0.7).tras(&Mat3::rot_x(-0.3));
        let p = v3(1.0, 2.0, 3.0);
        assert!((r.transpuesta().por(r.por(p)) - p).largo() < 1e-5);
    }
}
