// la escena del diorama. se divide en dos BVH:
//   estatica: todo lo que no se mueve (templo, jardin, techos). se
//             construye al arrancar y cada vez que cambia la estacion
//             (cambian el follaje, la nieve, el hielo del estanque...)
//   dinamica: lo que se anima o cambia con las interacciones (campana,
//             puertas, llamas, agua de la fuente, particulas). se
//             reconstruye cuando cambia el estado, y como son pocos
//             objetos cuesta muy poco
//   personaje: el personaje en miniatura que se mueve con las flechas.
//             va aparte para que sus rayos de choque no se toquen a el
//             mismo, y se rehace cada vez que camina
// ademas guarda la iluminacion del momento (sol/luna, cielo, exposicion).

pub mod jardin;
pub mod techo;
pub mod templo;

use crate::ambiente::{self, Estacion, Iluminacion, Paleta};
use crate::bvh::Bvh;
use crate::figuras::Aabb;
use crate::interaccion::EstadoDiorama;
use crate::matematica::v3;
use crate::personaje::Personaje;
use crate::render::Luz;

pub struct Escena {
    pub estatica: Bvh,
    pub dinamica: Bvh,
    pub personaje: Bvh,
    pub luces: Vec<Luz>,
    pub luz: Iluminacion,
    pub interior: Aabb,
    estacion: Estacion,
}

fn construir_estatica(estacion: Estacion) -> Bvh {
    let paleta = Paleta::de(estacion);
    let mut objetos = Vec::new();
    templo::crear_templo(&mut objetos, &paleta);
    jardin::crear_jardin(&mut objetos, &paleta);
    Bvh::construir(objetos)
}

impl Escena {
    pub fn nueva(estado: &EstadoDiorama) -> Escena {
        let a = &estado.ambiente;
        let mut escena = Escena {
            estatica: construir_estatica(a.estacion),
            dinamica: Bvh::construir(Vec::new()),
            personaje: Bvh::construir(Vec::new()),
            luces: Vec::new(),
            luz: ambiente::iluminacion(a.hora, a.estacion),
            interior: templo::interior(),
            estacion: a.estacion,
        };
        escena.actualizar(estado);
        escena
    }

    pub fn actualizar(&mut self, estado: &EstadoDiorama) {
        let a = &estado.ambiente;
        if a.estacion != self.estacion {
            self.estatica = construir_estatica(a.estacion);
            self.estacion = a.estacion;
        }
        self.luz = ambiente::iluminacion(a.hora, a.estacion);

        let mut objetos = Vec::new();
        // luz calida del altar, dentro del templo (con sombras)
        let mut luces = vec![Luz { pos: v3(0.0, 2.7, -7.6), color: v3(1.0, 0.68, 0.35) * 1.3, radio: 3.8, sombras: true }];
        templo::crear_puertas(&mut objetos, estado.apertura_puerta);
        jardin::crear_dinamicos(&mut objetos, &mut luces, estado, self.luz.noche);
        self.dinamica = Bvh::construir(objetos);
        self.luces = luces;
    }

    pub fn actualizar_personaje(&mut self, p: &Personaje) {
        let mut objetos = Vec::new();
        p.crear_objetos(&mut objetos);
        self.personaje = Bvh::construir(objetos);
    }

    pub fn cantidad_objetos(&self) -> usize {
        self.estatica.objetos.len() + self.dinamica.objetos.len() + self.personaje.objetos.len()
    }
}
