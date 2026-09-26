// techo japones curvo, armado con triangulos.
//
// la altura del techo en un punto depende de que tan lejos esta del
// alero mas cercano ("s" = 0 en el borde, 1 donde llega a la cumbrera o a
// la pared del piso de arriba):
//     h = y_alero + alto * s^curva + levante_en_esquinas
// con curva > 1 el techo queda casi plano en el alero y empinado arriba,
// la curva concava ("sori") tipica de la arquitectura japonesa. ademas
// las cuatro esquinas se levantan hacia arriba.
//
// sobre esa superficie se agregan: el canto del alero (tejas de borde +
// tabla de madera), los lomos de las limas (cumbreras diagonales), la
// cumbrera principal con sus remates y campanitas colgando en las puntas.

use crate::figuras::*;
use crate::material::*;
use crate::matematica::{v3, Mat3, Vec3};

pub struct Techo {
    pub cx: f32,
    pub cz: f32,
    pub y_alero: f32,
    // medio ancho del alero en X y en Z
    pub ax: f32,
    pub az: f32,
    // distancia horizontal desde el alero hasta donde termina la subida
    pub wx: f32,
    pub wz: f32,
    pub alto: f32,
    pub curva: f32,
    pub levante: f32,
    pub nx: usize,
    pub nz: usize,
    // anillo = techo de faldon que rodea un piso superior (sin cumbrera)
    pub anillo: bool,
    // material de la superficie: tejas, o nieve en invierno
    pub tejas: Material,
}

impl Techo {
    fn s_ejes(&self, x: f32, z: f32) -> (f32, f32) {
        let lx = x - self.cx;
        let lz = z - self.cz;
        (
            ((self.ax - lx.abs()) / self.wx).clamp(0.0, 1.0),
            ((self.az - lz.abs()) / self.wz).clamp(0.0, 1.0),
        )
    }

    // altura de la superficie del techo en (x, z) del mundo
    pub fn altura(&self, x: f32, z: f32) -> f32 {
        let (sx, sz) = self.s_ejes(x, z);
        let s = sx.min(sz);
        // el alero sube de a poco hacia las esquinas (nokizori)
        let esquina = ((1.0 - sx).powi(3)) * ((1.0 - sz).powi(3));
        self.y_alero + self.alto * s.powf(self.curva) + self.levante * esquina
    }

    fn normal(&self, x: f32, z: f32) -> Vec3 {
        let e = 0.02;
        let dx = (self.altura(x + e, z) - self.altura(x - e, z)) / (2.0 * e);
        let dz = (self.altura(x, z + e) - self.altura(x, z - e)) / (2.0 * e);
        v3(-dx, 1.0, -dz).normalizado()
    }

    // uv: la u corre a lo largo del alero (asi las columnas de tejas bajan
    // por la pendiente) y la v baja por la pendiente
    fn uv(&self, x: f32, z: f32) -> [f32; 2] {
        let (sx, sz) = self.s_ejes(x, z);
        if sx < sz {
            [z - self.cz, sx * self.wx]
        } else {
            [x - self.cx, sz * self.wz]
        }
    }

    fn punto(&self, x: f32, z: f32) -> Vec3 {
        v3(x, self.altura(x, z), z)
    }

    pub fn construir(&self, v: &mut Vec<Objeto>) {
        self.superficie(v);
        self.alero(v);
        self.limas(v);
        if !self.anillo {
            self.cumbrera(v);
        }
    }

    fn superficie(&self, v: &mut Vec<Objeto>) {
        let (nx, nz) = (self.nx, self.nz);
        let x_de = |i: usize| self.cx - self.ax + 2.0 * self.ax * i as f32 / nx as f32;
        let z_de = |j: usize| self.cz - self.az + 2.0 * self.az * j as f32 / nz as f32;

        for j in 0..nz {
            for i in 0..nx {
                let esquinas = [(x_de(i), z_de(j)), (x_de(i + 1), z_de(j)), (x_de(i + 1), z_de(j + 1)), (x_de(i), z_de(j + 1))];
                if self.anillo {
                    // en el faldon, la parte de adentro queda tapada por el
                    // piso superior: no hace falta generarla
                    let adentro = esquinas.iter().all(|&(x, z)| {
                        let (sx, sz) = self.s_ejes(x, z);
                        sx >= 1.0 && sz >= 1.0
                    });
                    if adentro {
                        continue;
                    }
                }
                let p: Vec<Vec3> = esquinas.iter().map(|&(x, z)| self.punto(x, z)).collect();
                let n: Vec<Vec3> = esquinas.iter().map(|&(x, z)| self.normal(x, z)).collect();
                let uv: Vec<[f32; 2]> = esquinas.iter().map(|&(x, z)| self.uv(x, z)).collect();
                for (a, b, c) in [(0, 1, 2), (0, 2, 3)] {
                    v.push(
                        triangulo([p[a], p[b], p[c]], [n[a], n[b], n[c]], [uv[a], uv[b], uv[c]], self.tejas)
                            .con_reverso(MADERA_OSCURA)
                            .con_relieve(Relieve::Tejas),
                    );
                }
            }
        }
    }

    // el canto del alero: una franja de tejas de borde y debajo la tabla
    // de madera, siguiendo el contorno (que sube en las esquinas)
    fn alero(&self, v: &mut Vec<Objeto>) {
        let mut contorno = Vec::new();
        let (x0, x1) = (self.cx - self.ax, self.cx + self.ax);
        let (z0, z1) = (self.cz - self.az, self.cz + self.az);
        for i in 0..self.nx {
            let t = |k: usize| x0 + (x1 - x0) * k as f32 / self.nx as f32;
            contorno.push(((t(i), z1), (t(i + 1), z1), v3(0.0, 0.0, 1.0)));
            contorno.push(((t(i + 1), z0), (t(i), z0), v3(0.0, 0.0, -1.0)));
        }
        for j in 0..self.nz {
            let t = |k: usize| z0 + (z1 - z0) * k as f32 / self.nz as f32;
            contorno.push(((x1, t(j + 1)), (x1, t(j)), v3(1.0, 0.0, 0.0)));
            contorno.push(((x0, t(j)), (x0, t(j + 1)), v3(-1.0, 0.0, 0.0)));
        }

        for ((xa, za), (xb, zb), n) in contorno {
            let a = self.punto(xa, za);
            let b = self.punto(xb, zb);
            let franjas = [(0.0, 0.09, CUMBRERA), (0.09, 0.24, MADERA)];
            for (arriba, abajo, mat) in franjas {
                let a0 = a - v3(0.0, arriba, 0.0);
                let b0 = b - v3(0.0, arriba, 0.0);
                let a1 = a - v3(0.0, abajo, 0.0);
                let b1 = b - v3(0.0, abajo, 0.0);
                let uv = [[xa + za, 0.0], [xb + zb, 0.0], [xb + zb, 0.15], [xa + za, 0.15]];
                v.push(triangulo([a0, b0, b1], [n; 3], [uv[0], uv[1], uv[2]], mat));
                v.push(triangulo([a0, b1, a1], [n; 3], [uv[0], uv[2], uv[3]], mat));
            }
        }
    }

    // lomos diagonales (limas) desde cada esquina hacia la cumbrera. se
    // arman con varios tramos para que sigan la curva del techo y tapan
    // el pliegue entre dos faldones
    fn limas(&self, v: &mut Vec<Objeto>) {
        for (sgx, sgz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            let pt = |q: f32| {
                let x = self.cx + sgx * (self.ax - q * self.wx);
                let z = self.cz + sgz * (self.az - q * self.wz);
                // el lomo se apoya sobre la superficie
                let mut p = self.punto(x, z) + v3(0.0, 0.1, 0.0);
                if q < 0.0 {
                    // la punta sale un poco mas alla y hacia arriba
                    p.y = self.altura(self.cx + sgx * self.ax, self.cz + sgz * self.az) + 0.1 - q * 1.2;
                }
                p
            };
            let pasos = [-0.035, 0.06, 0.16, 0.3, 0.5, 0.72, 1.0];
            for k in 0..pasos.len() - 1 {
                v.push(viga(pt(pasos[k]), pt(pasos[k + 1]), 0.26, 0.2, CUMBRERA));
            }
            // remate (onigawara) en la punta de la lima
            let punta = pt(-0.035);
            v.push(caja(punta + v3(0.0, 0.05, 0.0), v3(0.12, 0.16, 0.12), CUMBRERA));

            // campanita de bronce (furin) colgando de la esquina
            let esquina = v3(self.cx + sgx * (self.ax - 0.08), 0.0, self.cz + sgz * (self.az - 0.08));
            let y_esquina = self.altura(esquina.x, esquina.z) - 0.24;
            v.push(barra(v3(esquina.x, y_esquina, esquina.z), v3(esquina.x, y_esquina - 0.28, esquina.z), 0.012, METAL));
            v.push(cono(v3(esquina.x, y_esquina - 0.52, esquina.z), 0.1, 0.05, 0.24, METAL));
        }
    }

    // cumbrera principal a lo largo de X, con remates en los extremos y
    // adornos dorados (shachihoko) mirando hacia adentro
    fn cumbrera(&self, v: &mut Vec<Objeto>) {
        let largo = (self.ax - self.wx).max(0.0);
        let y = self.y_alero + self.alto;
        if largo < 0.05 {
            // techo piramidal: en la punta va una joya (hoju)
            v.push(cono(v3(self.cx, y - 0.05, self.cz), 0.22, 0.08, 0.35, CUMBRERA));
            v.push(cono(v3(self.cx, y + 0.3, self.cz), 0.1, 0.05, 0.25, ORO));
            v.push(esfera(v3(self.cx, y + 0.65, self.cz), 0.13, ORO));
            return;
        }
        let a = v3(self.cx - largo - 0.15, y + 0.14, self.cz);
        let b = v3(self.cx + largo + 0.15, y + 0.14, self.cz);
        v.push(viga(a, b, 0.42, 0.32, CUMBRERA));
        v.push(viga(a + v3(0.0, 0.2, 0.0), b + v3(0.0, 0.2, 0.0), 0.26, 0.1, CUMBRERA));
        for sg in [-1.0f32, 1.0] {
            let x = self.cx + sg * (largo + 0.2);
            // onigawara: placa alta en el extremo
            v.push(caja(v3(x, y + 0.32, self.cz), v3(0.1, 0.34, 0.3), CUMBRERA));
            // shachihoko: cuerpo, cabeza y cola levantada, en oro
            let px = x - sg * 0.05;
            v.push(elipsoide(v3(px, y + 0.8, self.cz), v3(0.14, 0.22, 0.12), ORO));
            v.push(esfera(v3(px + sg * 0.03, y + 0.66, self.cz), 0.13, ORO));
            v.push(cono(v3(px - sg * 0.1, y + 0.82, self.cz), 0.1, 0.02, 0.4, ORO).rotado(Mat3::rot_z(sg * 0.5)));
        }
    }
}

// pared de relleno ("friso") desde y_base hasta justo debajo del techo, a
// lo largo de la linea a-b. se parte en tramos para seguir la altura del
// techo, que cambia a lo largo de la pared
pub fn friso(v: &mut Vec<Objeto>, techo: &Techo, a: Vec3, b: Vec3, y_base: f32, grosor: f32, m: Material) {
    let tramos = 6;
    for k in 0..tramos {
        let p0 = a.lerp(b, k as f32 / tramos as f32);
        let p1 = a.lerp(b, (k + 1) as f32 / tramos as f32);
        let min = v3(p0.x.min(p1.x), y_base, p0.z.min(p1.z)) - v3(grosor, 0.0, grosor) * 0.5;
        let max = v3(p0.x.max(p1.x), 0.0, p0.z.max(p1.z)) + v3(grosor, 0.0, grosor) * 0.5;
        // el tope es el punto mas bajo del techo sobre las 4 esquinas del
        // tramo (el techo baja hacia afuera, asi que la cara exterior manda)
        let tope = [(min.x, min.z), (max.x, min.z), (min.x, max.z), (max.x, max.z)]
            .iter()
            .map(|&(x, z)| techo.altura(x, z))
            .fold(f32::MAX, f32::min)
            - 0.04;
        if tope <= y_base {
            continue;
        }
        let max = v3(max.x, tope, max.z);
        v.push(caja_mm(min, max, m));
    }
}
