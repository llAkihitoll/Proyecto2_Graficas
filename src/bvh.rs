// jerarquia de volumenes envolventes (BVH). con miles de triangulos en
// los techos, probar el rayo contra cada objeto seria carisimo. el BVH
// agrupa los objetos en cajas dentro de cajas: si el rayo no toca una
// caja, se saltea todo lo que tiene adentro de una sola vez.

use crate::figuras::{Aabb, Impacto, Objeto, Rayo, EPS};
use crate::matematica::{v3, Vec3};

const MAX_HOJA: usize = 4;

struct Nodo {
    caja: Aabb,
    // hoja: objetos [inicio, inicio + cantidad). interior: cantidad = 0 e
    // "inicio" es el indice del hijo izquierdo (el derecho es inicio + 1)
    inicio: u32,
    cantidad: u32,
}

pub struct Bvh {
    pub objetos: Vec<Objeto>,
    cajas: Vec<Aabb>,
    nodos: Vec<Nodo>,
}

impl Bvh {
    pub fn construir(objetos: Vec<Objeto>) -> Bvh {
        let mut bvh = Bvh { cajas: objetos.iter().map(|o| o.aabb()).collect(), objetos, nodos: Vec::new() };
        let n = bvh.objetos.len();
        if n == 0 {
            return bvh;
        }
        let mut indices: Vec<usize> = (0..n).collect();
        bvh.nodos.push(Nodo { caja: Aabb::VACIA, inicio: 0, cantidad: 0 });
        bvh.dividir(0, &mut indices, 0);

        // reordenamos los objetos para que cada hoja quede contigua
        let objetos: Vec<Objeto> = indices.iter().map(|&i| bvh.objetos[i]).collect();
        let cajas: Vec<Aabb> = indices.iter().map(|&i| bvh.cajas[i]).collect();
        bvh.objetos = objetos;
        bvh.cajas = cajas;
        bvh
    }

    // divide el rango de indices por la mediana del eje mas largo
    fn dividir(&mut self, nodo: usize, indices: &mut [usize], desplazamiento: usize) {
        let mut caja = Aabb::VACIA;
        let mut caja_centros = Aabb::VACIA;
        for &i in indices.iter() {
            caja = caja.unir(&self.cajas[i]);
            caja_centros = caja_centros.agregar(self.cajas[i].centro());
        }
        self.nodos[nodo].caja = caja;

        if indices.len() <= MAX_HOJA {
            self.nodos[nodo].inicio = desplazamiento as u32;
            self.nodos[nodo].cantidad = indices.len() as u32;
            return;
        }

        let tam = caja_centros.max - caja_centros.min;
        let eje = if tam.x >= tam.y && tam.x >= tam.z {
            0
        } else if tam.y >= tam.z {
            1
        } else {
            2
        };
        let cajas = &self.cajas;
        indices.sort_unstable_by(|&a, &b| {
            cajas[a].centro().eje(eje).partial_cmp(&cajas[b].centro().eje(eje)).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mitad = indices.len() / 2;

        let izq = self.nodos.len();
        self.nodos.push(Nodo { caja: Aabb::VACIA, inicio: 0, cantidad: 0 });
        self.nodos.push(Nodo { caja: Aabb::VACIA, inicio: 0, cantidad: 0 });
        self.nodos[nodo].inicio = izq as u32;
        self.nodos[nodo].cantidad = 0;

        let (a, b) = indices.split_at_mut(mitad);
        self.dividir(izq, a, desplazamiento);
        self.dividir(izq + 1, b, desplazamiento + mitad);
    }

    fn inversa(d: Vec3) -> Vec3 {
        let f = |x: f32| if x.abs() > 1e-12 { 1.0 / x } else { 1e12_f32.copysign(x) };
        v3(f(d.x), f(d.y), f(d.z))
    }

    // impacto mas cercano
    pub fn intersectar(&self, r: &Rayo, t_max: f32) -> Option<Impacto<'_>> {
        if self.nodos.is_empty() {
            return None;
        }
        let inv = Self::inversa(r.dir);
        let mut mejor: Option<Impacto> = None;
        let mut limite = t_max;
        let mut pila = [0u32; 64];
        let mut tope = 1;
        pila[0] = 0;

        while tope > 0 {
            tope -= 1;
            let nodo = &self.nodos[pila[tope] as usize];
            if nodo.caja.choca(r.origen, inv, limite).is_none() {
                continue;
            }
            if nodo.cantidad > 0 {
                let ini = nodo.inicio as usize;
                for i in ini..ini + nodo.cantidad as usize {
                    if self.cajas[i].choca(r.origen, inv, limite).is_none() {
                        continue;
                    }
                    if let Some(imp) = self.objetos[i].intersectar(r, limite) {
                        limite = imp.t;
                        mejor = Some(imp);
                    }
                }
            } else {
                // visitamos primero el hijo mas cercano, asi "limite" se
                // achica antes y el otro hijo se descarta mas seguido
                let izq = nodo.inicio;
                let der = nodo.inicio + 1;
                let ti = self.nodos[izq as usize].caja.choca(r.origen, inv, limite);
                let td = self.nodos[der as usize].caja.choca(r.origen, inv, limite);
                match (ti, td) {
                    (Some(a), Some(b)) => {
                        let (cerca, lejos) = if a <= b { (izq, der) } else { (der, izq) };
                        pila[tope] = lejos;
                        pila[tope + 1] = cerca;
                        tope += 2;
                    }
                    (Some(_), None) => {
                        pila[tope] = izq;
                        tope += 1;
                    }
                    (None, Some(_)) => {
                        pila[tope] = der;
                        tope += 1;
                    }
                    _ => {}
                }
            }
        }
        mejor
    }

    // cuanta luz llega a traves del segmento (1 = nada en el medio, 0 =
    // sombra total). los objetos transparentes dejan pasar una parte
    pub fn transmision(&self, r: &Rayo, t_max: f32) -> f32 {
        if self.nodos.is_empty() {
            return 1.0;
        }
        let inv = Self::inversa(r.dir);
        let mut paso = 1.0;
        let mut pila = [0u32; 64];
        let mut tope = 1;
        pila[0] = 0;
        while tope > 0 {
            tope -= 1;
            let nodo = &self.nodos[pila[tope] as usize];
            if nodo.caja.choca(r.origen, inv, t_max).is_none() {
                continue;
            }
            if nodo.cantidad > 0 {
                let ini = nodo.inicio as usize;
                for i in ini..ini + nodo.cantidad as usize {
                    let obj = &self.objetos[i];
                    if !obj.proyecta_sombra || self.cajas[i].choca(r.origen, inv, t_max).is_none() {
                        continue;
                    }
                    if obj.intersectar(r, t_max).is_some() {
                        let tr = obj.material.transparencia;
                        if tr <= 0.0 {
                            return 0.0;
                        }
                        paso *= tr;
                        if paso < 0.02 {
                            return 0.0;
                        }
                    }
                }
            } else {
                pila[tope] = nodo.inicio;
                pila[tope + 1] = nodo.inicio + 1;
                tope += 2;
            }
        }
        paso
    }

    // para la camara: hay algun objeto cuya caja contenga este punto?
    pub fn contiene(&self, p: Vec3, margen: f32) -> bool {
        self.cajas.iter().any(|c| c.contiene(p, margen))
    }
}

// para evitar el "acne" de sombras: separamos un poco el origen del
// rayo secundario de la superficie
pub fn desplazar(p: Vec3, n: Vec3) -> Vec3 {
    p + n * (EPS * 4.0)
}
