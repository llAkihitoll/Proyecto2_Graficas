// el raytracer: por cada pixel se lanza un rayo desde la camara, y en
// cada superficie que toca se calcula
//   - luz local: ambiente + difusa (Lambert) + especular (Blinn-Phong),
//     con rayos de sombra hacia el sol y las luces puntuales
//   - reflexion: un rayo secundario en la direccion espejo
//   - refraccion: un rayo secundario que atraviesa la superficie y se
//     desvia segun la ley de Snell (agua, vidrio)
// y se mezclan segun los parametros del material y el termino de Fresnel.
// si un rayo no choca con nada, toma el color del skybox (mezcla de dos
// momentos del dia) mas el sol y la luna en su posicion real.

use crate::bvh::desplazar;
use crate::camara::Vista;
use crate::escena::Escena;
use crate::figuras::{Impacto, Interactivo, Rayo, Relieve};
use crate::matematica::{reflejar, refractar, schlick, suavizar, v3, Vec3};
use crate::skybox::Skybox;
use crate::textura::Texturas;
use std::f32::consts::PI;

// cuantos rebotes seguimos como maximo (reflexion + refraccion)
const MAX_PROF: u32 = 5;
// si la contribucion de un rayo secundario al pixel es menor, no se traza
const PESO_MIN: f32 = 0.02;

pub struct Recursos {
    pub texturas: Texturas,
    // un skybox por momento del dia (ver ambiente.rs)
    pub cielos: Vec<Skybox>,
}

// luz puntual (linternas, altar). "radio" es hasta donde alcanza: afuera
// de ese radio ni se calcula, asi no cuesta nada lejos de la luz
#[derive(Clone, Copy, Debug)]
pub struct Luz {
    pub pos: Vec3,
    pub color: Vec3,
    pub radio: f32,
    pub sombras: bool,
}

struct Ctx<'a> {
    escena: &'a Escena,
    rec: &'a Recursos,
}

impl<'a> Ctx<'a> {
    fn intersectar(&self, r: &Rayo, t_max: f32) -> Option<Impacto<'a>> {
        let a = self.escena.estatica.intersectar(r, t_max);
        let limite = a.as_ref().map_or(t_max, |i| i.t);
        let b = self.escena.dinamica.intersectar(r, limite);
        let limite = b.as_ref().map_or(limite, |i| i.t);
        let c = self.escena.personaje.intersectar(r, limite);
        c.or(b).or(a)
    }

    fn transmision(&self, r: &Rayo, t_max: f32) -> f32 {
        let a = self.escena.estatica.transmision(r, t_max);
        if a <= 0.0 {
            return 0.0;
        }
        let b = a * self.escena.dinamica.transmision(r, t_max);
        if b <= 0.0 {
            return 0.0;
        }
        b * self.escena.personaje.transmision(r, t_max)
    }

    fn trazar(&self, r: &Rayo, prof: u32, peso: f32, medio: Option<Vec3>) -> Vec3 {
        let imp = match self.intersectar(r, f32::INFINITY) {
            Some(i) => i,
            None => return self.cielo(r.dir),
        };

        let color = self.sombrear(r, &imp, prof, peso, medio);

        // si el rayo viajaba dentro de agua/vidrio, el medio absorbe parte
        // del color segun la distancia recorrida (ley de Beer-Lambert)
        match medio {
            Some(abs) => color.mul(v3((-abs.x * imp.t).exp(), (-abs.y * imp.t).exp(), (-abs.z * imp.t).exp())),
            None => color,
        }
    }

    // color del cielo en la direccion d
    fn cielo(&self, d: Vec3) -> Vec3 {
        let il = &self.escena.luz;
        let (a, b, t) = il.mezcla;
        let mut c = if t < 0.001 {
            self.rec.cielos[a].muestrear(d)
        } else if t > 0.999 {
            self.rec.cielos[b].muestrear(d)
        } else {
            self.rec.cielos[a].muestrear(d).lerp(self.rec.cielos[b].muestrear(d), t)
        };
        // el sol y la luna se esconden detras del horizonte
        let sobre_horizonte = suavizar(-0.01, 0.02, d.y);
        if sobre_horizonte <= 0.0 {
            return c;
        }

        // sol: disco brillante y halo del color de su luz
        let cs = d.dot(il.sol_dir);
        if cs > 0.0 {
            let visible = suavizar(-0.03, 0.03, il.sol_dir.y) * sobre_horizonte;
            let disco = suavizar(0.9993, 0.9996, cs);
            let halo = il.sol_color * (cs.powf(60.0) * 0.4 + cs.powf(600.0) * 0.8);
            c += (v3(1.7, 1.5, 1.2) * disco + halo) * visible;
        }

        // luna: disco palido con halo azulado, mas notoria de noche
        let cl = d.dot(il.luna_dir);
        if cl > 0.0 {
            let visible = il.luna_visible * (0.35 + 0.65 * il.noche) * sobre_horizonte;
            let disco = suavizar(0.99955, 0.9998, cl);
            // manchas (mares lunares) con un patron simple
            let p = d - il.luna_dir;
            let manchas = 1.0 - 0.18 * (((p.x * 900.0).sin() * (p.z * 700.0 + p.y * 500.0).cos()) * 0.5 + 0.5);
            c += (v3(1.1, 1.1, 1.2) * (disco * manchas) + v3(0.3, 0.35, 0.5) * (cl.powf(400.0) * 0.4)) * visible;
        }
        c
    }

    fn sombrear(&self, r: &Rayo, imp: &Impacto, prof: u32, peso: f32, medio: Option<Vec3>) -> Vec3 {
        let obj = imp.objeto;
        let d = r.dir;
        let de_frente = d.dot(imp.normal_geo) < 0.0;

        let mut m = obj.material;
        if !de_frente {
            if let Some(rev) = obj.reverso {
                m = rev;
            }
        }

        let uv = (imp.uv.0 * m.escala_uv, imp.uv.1 * m.escala_uv);
        let mut n = aplicar_relieve(imp.normal, imp.punto, uv, obj.relieve);

        // las normales se dejan mirando hacia el lado de donde viene el rayo
        let mut ng = imp.normal_geo;
        if !de_frente {
            n = -n;
            ng = -ng;
        }
        if n.dot(d) > 0.0 {
            // el relieve no puede dar vuelta la normal respecto al rayo
            n = ng;
        }

        let base = match m.textura {
            Some(i) => self.rec.texturas.lista[i].muestrear(uv.0, uv.1).mul(m.albedo),
            None => m.albedo,
        };

        let p = imp.punto;
        let p_afuera = desplazar(p, ng);

        // ---------- luz local ----------
        let (difusa, especular) = self.iluminar(p_afuera, n, d, &m, base);

        let reflectividad = m.reflectividad;
        let transparencia = m.transparencia;
        let mut peso_local = (1.0 - reflectividad - transparencia).max(0.0);
        let mut peso_refl = reflectividad;
        let mut peso_refr = 0.0;
        let mut dir_refr = None;
        let mut medio_refr = None;

        // ---------- refraccion + Fresnel ----------
        if transparencia > 0.0 {
            let (n1, n2) = if de_frente { (1.0, m.ior) } else { (m.ior, 1.0) };
            let cos_i = (-d.dot(n)).clamp(0.0, 1.0);
            match refractar(d, n, n1 / n2) {
                Some(t) => {
                    // al salir de un medio mas denso, Schlick se evalua
                    // con el angulo del lado de menor indice
                    let cos_f = if n1 > n2 { (-t.dot(n)).abs() } else { cos_i };
                    let kr = schlick(cos_f, n1, n2);
                    peso_refl += transparencia * kr;
                    peso_refr = transparencia * (1.0 - kr);
                    dir_refr = Some(t);
                    medio_refr = if de_frente { Some(m.absorcion) } else { None };
                }
                None => {
                    // reflexion interna total: todo lo que iba a pasar se refleja
                    peso_refl += transparencia;
                }
            }
            if m.ior <= 1.001 {
                // papel: deja pasar luz pero sin desviar ni reflejar extra
                peso_local = 1.0 - transparencia;
            }
        }

        let mut color = difusa * peso_local + especular + m.emision;

        if prof >= MAX_PROF {
            // sin rebotes: aproximamos lo que faltaba con el cielo
            return color + self.cielo(reflejar(d, n)) * (peso_refl + peso_refr) * 0.5;
        }

        // ---------- reflexion ----------
        if peso_refl * peso > PESO_MIN {
            let rd = reflejar(d, n).normalizado();
            let rr = Rayo::nuevo(p_afuera, rd);
            // el rayo reflejado sigue en el mismo medio en el que venia
            let mut c = self.trazar(&rr, prof + 1, peso * peso_refl, medio);
            // los metales tinen su reflejo con su propio color
            if transparencia == 0.0 && reflectividad > 0.3 {
                c = c.mul((base * 1.5).min(Vec3::UNO));
            }
            color += c * peso_refl;
        }

        // ---------- refraccion ----------
        if let Some(t) = dir_refr {
            if peso_refr * peso > PESO_MIN {
                let rr = Rayo::nuevo(desplazar(p, -ng), t.normalizado());
                let medio_nuevo = if m.ior <= 1.001 { medio } else { medio_refr };
                color += self.trazar(&rr, prof + 1, peso * peso_refr, medio_nuevo) * peso_refr;
            }
        }

        color
    }

    // devuelve (difusa, especular). la difusa todavia se multiplica por
    // el peso local del material; el brillo especular se suma entero
    fn iluminar(&self, p: Vec3, n: Vec3, d: Vec3, m: &crate::material::Material, base: Vec3) -> (Vec3, Vec3) {
        let escena = self.escena;

        // luz ambiente de "hemisferio": las caras que miran al cielo
        // reciben mas luz violeta, las que miran al piso un rebote calido
        let mut ambiente = escena.luz.amb_suelo.lerp(escena.luz.amb_cielo, n.y * 0.5 + 0.5);
        if escena.interior.contiene(p, 0.0) {
            ambiente *= 0.3; // adentro del templo casi no entra luz del cielo
        }
        let mut difusa = base.mul(ambiente);
        let mut especular = Vec3::CERO;

        // sol o luna (luz direccional con sombra)
        let l = escena.luz.luz_dir;
        let ndl = n.dot(l);
        if ndl > 0.0 && escena.luz.luz_color.luminancia() > 1e-3 {
            let tr = self.transmision(&Rayo::nuevo(p, l), f32::INFINITY);
            if tr > 0.0 {
                let luz = escena.luz.luz_color * tr;
                difusa += base.mul(luz) * ndl;
                let h = (l - d).normalizado();
                especular += luz * (m.especular * n.dot(h).max(0.0).powf(m.brillo));
            }
        }

        // luces puntuales
        for luz in &escena.luces {
            let hacia = luz.pos - p;
            let dist = hacia.largo();
            if dist > luz.radio || dist < 1e-4 {
                continue;
            }
            let l = hacia / dist;
            let ndl = n.dot(l);
            if ndl <= 0.0 {
                continue;
            }
            let caida = (1.0 - dist / luz.radio).powi(2);
            let mut c = luz.color * caida;
            if luz.sombras {
                let tr = self.transmision(&Rayo::nuevo(p, l), dist - 0.05);
                if tr <= 0.0 {
                    continue;
                }
                c *= tr;
            }
            difusa += base.mul(c) * ndl;
            let h = (l - d).normalizado();
            especular += c * (m.especular * n.dot(h).max(0.0).powf(m.brillo));
        }

        (difusa, especular)
    }
}

// perturba la normal para dar relieve (tejas, ondas del agua)
fn aplicar_relieve(n: Vec3, p: Vec3, uv: (f32, f32), relieve: Relieve) -> Vec3 {
    match relieve {
        Relieve::Ninguno => n,
        Relieve::Tejas => {
            // direccion horizontal que cruza la pendiente del techo
            let t = n.cross(Vec3::ARRIBA);
            if t.largo() < 0.1 {
                return n;
            }
            let fase = (uv.0 * 8.0).fract();
            let fase = if fase < 0.0 { fase + 1.0 } else { fase };
            (n + t.normalizado() * (0.45 * (PI * fase).cos())).normalizado()
        }
        Relieve::Ondas { cx, cz, amplitud, tiempo, radial } => {
            if n.y < 0.5 || amplitud <= 0.0 {
                return n;
            }
            // gradiente de la altura de las ondas: la normal se inclina en
            // contra de la pendiente
            let (gx, gz) = if radial {
                let dx = p.x - cx;
                let dz = p.z - cz;
                let r = (dx * dx + dz * dz).sqrt().max(1e-3);
                let k = 11.0;
                let g = amplitud * (k * r - 7.0 * tiempo).cos() * (-0.6 * r).exp();
                (g * dx / r, g * dz / r)
            } else {
                let ondas = [(0.8f32, 0.6f32, 2.3f32, 0.9f32), (-0.5, 0.86, 3.7, 1.3), (0.2, -0.98, 5.9, 2.1)];
                let mut gx = 0.0;
                let mut gz = 0.0;
                for (dx, dz, k, w) in ondas {
                    let c = (k * (dx * p.x + dz * p.z) + w * tiempo).cos() / k.sqrt();
                    gx += c * dx;
                    gz += c * dz;
                }
                (gx * amplitud, gz * amplitud)
            };
            (n - v3(gx, 0.0, gz)).normalizado()
        }
    }
}

// compresion suave de los brillos para que las luces fuertes no se
// "quemen" de golpe al pasar de 1.0
#[inline]
fn tonos(x: f32) -> u8 {
    let y = if x < 0.8 { x.max(0.0) } else { 0.8 + 0.2 * (1.0 - (-(x - 0.8) / 0.2).exp()) };
    (y * 255.0 + 0.5) as u8
}

// renderiza la escena completa en "buffer" (RGBA, ancho x alto),
// repartiendo bloques de filas entre todos los nucleos del procesador
pub fn renderizar(escena: &Escena, rec: &Recursos, vista: &Vista, ancho: usize, alto: usize, buffer: &mut [u8]) {
    let ctx = Ctx { escena, rec };
    let exposicion = escena.luz.exposicion;
    let hilos = std::thread::available_parallelism().map_or(4, |n| n.get());
    const FILAS: usize = 4;

    let mut trabajo: Vec<Vec<(usize, &mut [u8])>> = (0..hilos).map(|_| Vec::new()).collect();
    for (i, bloque) in buffer.chunks_mut(ancho * 4 * FILAS).enumerate() {
        trabajo[i % hilos].push((i * FILAS, bloque));
    }

    std::thread::scope(|s| {
        for bloques in trabajo {
            let ctx = &ctx;
            s.spawn(move || {
                for (fila0, bloque) in bloques {
                    for (k, pixel) in bloque.chunks_mut(4).enumerate() {
                        let x = k % ancho;
                        let y = fila0 + k / ancho;
                        let rayo = vista.rayo(x as f32 + 0.5, y as f32 + 0.5, ancho as f32, alto as f32);
                        let c = ctx.trazar(&rayo, 0, 1.0, None) * exposicion;
                        pixel[0] = tonos(c.x);
                        pixel[1] = tonos(c.y);
                        pixel[2] = tonos(c.z);
                        pixel[3] = 255;
                    }
                }
            });
        }
    });
}

// que objeto interactivo hay bajo este rayo (para el click o la tecla E)
pub fn elegir(escena: &Escena, rec: &Recursos, rayo: &Rayo) -> Option<(Interactivo, f32)> {
    let ctx = Ctx { escena, rec };
    ctx.intersectar(rayo, f32::INFINITY).map(|i| (i.objeto.id, i.t))
}
