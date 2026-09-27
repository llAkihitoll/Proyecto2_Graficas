// diorama interactivo de un templo japones con raytracing propio.
// raylib solo se usa para la ventana, el teclado/mouse y para leer y
// guardar imagenes PNG; todo el trazado de rayos esta escrito aca.

pub mod ambiente;
pub mod bvh;
pub mod camara;
pub mod escena;
pub mod figuras;
pub mod generador;
pub mod generador_sonidos;
pub mod interaccion;
pub mod material;
pub mod matematica;
pub mod render;
pub mod ruido;
pub mod skybox;
pub mod sonido;
pub mod textura;

// raylib imprime una linea por cada archivo que carga o guarda; dejamos
// solo las advertencias y errores
pub fn silenciar_logs() {
    unsafe {
        raylib::ffi::SetTraceLogLevel(raylib::ffi::TraceLogLevel::LOG_WARNING as i32);
    }
}
