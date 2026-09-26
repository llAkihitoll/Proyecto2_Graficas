// la escena del diorama. se divide en dos BVH:
//   estatica: todo lo que nunca cambia (templo, jardin, techos). se
//             construye una sola vez al arrancar
//   dinamica: lo que se anima o cambia con las interacciones (campana,
//             puertas, llamas, agua de la fuente). se reconstruye cuando
//             cambia el estado, y como son pocos objetos cuesta muy poco

pub mod jardin;
pub mod techo;
pub mod templo;

use crate::bvh::Bvh;
use crate::figuras::Aabb;
use crate::interaccion::EstadoDiorama;
use crate::matematica::{v3, Vec3};
use crate::render::Luz;
use crate::skybox;

pub struct Escena {
    pub estatica: Bvh,
    pub dinamica: Bvh,
    pub luces: Vec<Luz>,
    pub dir_sol: Vec3,
    pub interior: Aabb,
}

impl Escena {
    pub fn nueva(estado: &EstadoDiorama) -> Escena {
        let mut objetos = Vec::new();
        templo::crear_templo(&mut objetos);
        jardin::crear_jardin(&mut objetos);
        let mut escena = Escena {
            estatica: Bvh::construir(objetos),
            dinamica: Bvh::construir(Vec::new()),
            luces: Vec::new(),
            dir_sol: skybox::direccion_sol(),
            interior: templo::interior(),
        };
        escena.actualizar(estado);
        escena
    }

    pub fn actualizar(&mut self, estado: &EstadoDiorama) {
        let mut objetos = Vec::new();
        // luz calida del altar, dentro del templo (con sombras)
        let mut luces = vec![Luz { pos: v3(0.0, 2.7, -7.6), color: v3(1.0, 0.68, 0.35) * 1.3, radio: 3.8, sombras: true }];
        templo::crear_puertas(&mut objetos, estado.apertura_puerta);
        jardin::crear_dinamicos(&mut objetos, &mut luces, estado);
        self.dinamica = Bvh::construir(objetos);
        self.luces = luces;
    }

    pub fn cantidad_objetos(&self) -> usize {
        self.estatica.objetos.len() + self.dinamica.objetos.len()
    }
}
