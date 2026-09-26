# Diorama interactivo: templo japonés con raytracing

Diorama 3D de un jardín japonés al atardecer con un templo de dos niveles
como punto focal, renderizado **con un raytracer propio escrito en Rust**.
[raylib](https://www.raylib.com/) solo se usa para abrir la ventana, leer
el teclado/mouse y leer/guardar imágenes PNG: la intersección de rayos,
materiales, texturas, luces, sombras, reflexión, refracción y skybox
están implementados a mano.

![Vista general](capturas/01_vista_general.png)

## Video de demostración

> Coloca aquí el link del video de YouTube grabado para la entrega:

- **Video:** _(pendiente)_

## Cómo correrlo

```
cargo run --release
```

La primera vez se generan las texturas (`assets/textures`) y el skybox
(`assets/skybox`) si no existen (tarda unos segundos). Para regenerarlos:

```
cargo run --release --bin generar_texturas
```

También hay un renderizador sin ventana, útil para sacar imágenes:

```
cargo run --release --bin captura -- <vista 1-5> <etapa 0-4> <ancho> <alto> <archivo.png>
```

Y pruebas de la matemática (Snell, reflexión interna total, rotaciones) y
de la progresión de interacciones:

```
cargo test --release
```

### Notas de compilación en esta máquina

- `.cargo/config.toml` fija `LIBCLANG_PATH` a la instalación de MSYS2
  (bindgen de `raylib-sys` no la encuentra solo). En otra PC puede que
  haya que ajustar o borrar esa ruta.
- Compilar desde PowerShell o la terminal de VS Code. En *Git Bash* el
  `PATH` pone primero el MinGW de Git (`/mingw64/bin`), y el `gcc` de
  MSYS2 carga DLLs incompatibles y falla al compilar raylib.

## Controles

| Acción | Tecla / mouse |
|---|---|
| Rotar la cámara alrededor del diorama | flechas o arrastrar con clic izquierdo |
| Zoom | rueda del mouse o `+` / `-` |
| Mover el punto que mira la cámara | `W` `A` `S` `D` |
| Subir / bajar el punto que mira | `Q` / `Z` |
| Interactuar con el objeto bajo el mouse | clic izquierdo (sin arrastrar) |
| Interactuar con el objeto en el centro (mira) | `E` |
| Campana / linternas / fuente / puerta | `B` / `L` / `F` / `T` |
| Vistas predefinidas | `1` general, `2` templo, `3` estanque, `4` techo, `5` patio |
| Reiniciar cámara | `R` |
| Captura de pantalla (`captura.png`) | `P` |
| Mostrar/ocultar ayuda | `H` |

Al pasar el mouse sobre un objeto interactivo aparece su nombre (y si
todavía está bloqueado).

## Recorrido (progresión)

Cada interacción se **desbloquea al completar la anterior**; una vez
desbloqueada se puede usar libremente. El HUD muestra el objetivo actual.

```
Explorar el jardín → tocar la CAMPANA (B)
  → encender las LINTERNAS (L)
    → seguir el camino y activar la FUENTE (F)
      → abrir la PUERTA del templo (T) y ver el altar
```

| Interacción | Qué pasa |
|---|---|
| 🔔 Campana | El mazo de madera se acerca y golpea; la campana oscila alrededor de su punto de suspensión (oscilación amortiguada) y el bronce brilla y se apaga de a poco. |
| 🏮 Linternas | Las 4 linternas de piedra encienden su llama (material emisivo, visible refractada a través del vidrio) y las 2 linternas de papel del alero se iluminan. Cada una agrega una luz puntual cálida que ilumina el entorno. |
| ⛲ Fuente | El nivel del agua sube, aparecen ondas circulares animadas (perturbación de la normal) y chorros de gotas que caen del cuenco superior. Con más agua se ven mejor las monedas refractadas en el fondo. |
| 🚪 Puerta | Las dos hojas giran sobre sus bisagras hacia adentro y dejan ver el altar dorado iluminado por velas. |

![Todo activo](capturas/06_todo_activo.png)

## Técnicas de raytracing

- **Un rayo por pixel** desde una cámara en perspectiva; se busca el
  impacto más cercano con una **BVH** (jerarquía de cajas envolventes)
  para que miles de triángulos del techo no hagan lento el render.
- **Primitivas**: caja (slab test), esfera/elipsoide, cilindro/tronco de
  cono (cuadrática + tapas) y triángulo (Möller–Trumbore). Cada objeto
  tiene posición, rotación y escala propias; el rayo se lleva a su
  espacio local para intersectarlo.
- **Iluminación**: ambiente de hemisferio + difusa (Lambert) + especular
  (Blinn-Phong), sol direccional con **rayos de sombra** (los objetos
  transparentes dejan pasar parte de la luz) y luces puntuales con radio
  de alcance.
- **Reflexión**: rayo secundario en la dirección `r = d - 2(d·n)n`.
  Los metales tiñen el reflejo con su color.
- **Refracción**: ley de Snell con **reflexión interna total**, reparto
  reflexión/refracción con **Fresnel (Schlick)** y **absorción de Beer**
  dentro del medio (le da al agua su color turquesa según la
  profundidad). Hasta 5 rebotes, cortando rayos que aportan menos de 2%.
- **Texturas**: PNG cargados desde `assets/textures`, con muestreo
  bilineal y repetición. Proyección por caja, cilíndrica o uv propias
  (techo).
- **Relieve**: perturbación de la normal para las tejas y para las
  ondas del agua (estanque y fuente).
- **Skybox**: cubemap de 6 imágenes (`assets/skybox`), atardecer con
  sol, nubes, estrellas, montañas y el monte Fuji detrás del templo. La
  dirección del sol del cielo es la misma que la de la luz principal.

### Materiales principales

Cada material tiene textura propia y parámetros propios (`src/material.rs`):

| Material | Textura | Albedo | Especular (brillo) | Reflectividad | Transparencia | IOR | Uso |
|---|---|---|---|---|---|---|---|
| **Madera** | `wood.png` | 1.0 | 0.25 (24) | 0.03 | 0 | – | pisos, puertas, paredes, puente |
| **Piedra** | `stone.png` | 1.0 | 0.05 (8) | 0.02 | 0 | – | base, escaleras, camino, linternas, fuente |
| **Metal (bronce)** | `metal.png` | 1.0 | 0.9 (90) | 0.45 | 0 | – | campana, techos de linternas, adornos |
| **Agua** | `water.png` | 1.0 | 0.9 (160) | 0.12 + Fresnel | 0.8 | 1.33 | estanque, fuente, gotas |
| **Vidrio** | `glass.png` | 1.0 | 1.0 (220) | 0.06 + Fresnel | 0.88 | 1.5 | ventanas, linternas, esfera de cristal |

Variantes de apoyo: laca bermellón y negra, madera oscura, oro, tejas
(`roof_tiles.png`), yeso y papel shoji translúcido (`paper.png`), pasto,
grava rastrillada, corteza, follaje (pino, arce, sakura, azalea), koi,
llama emisiva.

![Estanque](capturas/03_estanque_reflejos.png)

## El templo

Salón de dos niveles construido en `src/escena/templo.rs` con funciones
reutilizables:

| Función | Contenido |
|---|---|
| `crear_base` | base de piedra de 2 escalones con cornisa |
| `crear_escaleras` | 6 escalones + pasamanos lacados con remates dorados |
| `crear_veranda` / `crear_barandales` | veranda de madera y barandal perimetral (postes + 2 travesaños) |
| `crear_columnas` | 12 columnas bermellón con base negra y anillo dorado |
| `crear_muros` | paneles shoji con celosía, travesaño de vidrio sobre la puerta, paredes de tablas |
| `crear_ventanas` | ventanas laterales de vidrio con marco y celosía |
| `crear_vigas` | vigas *nageshi* y *kashiragi* alrededor del salón |
| `crear_mensulas` | ménsulas *tokyō* sobre cada columna del borde y viga del alero |
| `crear_piso_superior` | segundo nivel con columnas, paredes de yeso y ventanas de celosía |
| `crear_techos` | techo de faldón + techo principal + frisos |
| `crear_decoraciones` | placa con marco dorado, caja de ofrendas, campanita con cuerda |
| `crear_altar` | altar con figura dorada, aureola y velas |
| `crear_puertas` | puerta doble con marco, clavos y tirador (animada) |

**Techos** (`src/escena/techo.rs`): superficie de triángulos cuya altura
sigue la curva cóncava japonesa (*sori*, `s^curva`), con el alero que se
levanta hacia las esquinas (*nokizori*). Encima: canto del alero (tejas
de borde + tabla de madera), limas diagonales en tramos que siguen la
curva, cumbrera con *onigawara* y *shachihoko* dorados, y campanitas
*fūrin* en cada esquina. La cara de abajo del techo usa madera. El mismo
generador arma el techo del campanario.

![Techo](capturas/04_techo.png)

## Composición del jardín

```
            pino                          bambú
   arce             TEMPLO (+ altar)          pino
        linterna    escalera    linterna
  CAMPANARIO     patio de grava       FUENTE
  sakura  ~~~~~~~ PUENTE ARQUEADO ~~~~~~~  arce
          ~~~ ESTANQUE (koi, lotos) ~~~
        linterna     camino     linterna
                     TORII
```

El diorama es una isla flotante sobre un mar de nubes, con el templo al
fondo, el camino que lleva la mirada desde el torii, y el estanque con el
puente dando profundidad. La vegetación queda en los bordes.

## Rendimiento

- Render en paralelo por bloques de filas (`std::thread::scope`, sin
  crates extra).
- **Resolución progresiva**: 320×200 mientras la cámara se mueve o hay
  animaciones grandes, 512×320 mientras solo se anima el agua y 880×550
  cuando todo está quieto (se dibuja una vez y se reutiliza).
- La escena estática (~6500 primitivas, la mayoría triángulos del techo)
  se arma una sola vez; solo se reconstruye la parte dinámica (~80
  objetos).
- En un procesador de 12 hilos lógicos: ~110–330 ms la imagen en alta,
  y ~20–55 ms la previa (320×200).

## Estructura

```
src/
├── main.rs            ventana, input, render progresivo y HUD
├── lib.rs
├── matematica.rs      Vec3, Mat3, reflejar, refractar (Snell), Schlick
├── figuras.rs         Rayo, Aabb, primitivas e intersecciones
├── bvh.rs             BVH: impacto más cercano y rayos de sombra
├── material.rs        materiales
├── textura.rs         carga y muestreo de texturas
├── skybox.rs          cubemap y dirección del sol
├── camara.rs          cámara orbital, vistas predefinidas
├── render.rs          trazado recursivo, luces, reflexión, refracción
├── interaccion.rs     estado, progresión y animaciones
├── ruido.rs           ruido procedural (para generar assets)
├── generador.rs       genera texturas y skybox en PNG
├── escena/
│   ├── mod.rs         escena estática + dinámica
│   ├── templo.rs      el templo
│   ├── techo.rs       generador de techos curvos
│   └── jardin.rs      terreno, estanque, puente, torii, campanario,
│                      fuente, linternas, vegetación
└── bin/
    ├── generar_texturas.rs
    └── captura.rs
assets/
├── textures/          wood, stone, metal, water, glass, roof_tiles,
│                      paper, leaves, grass, gravel, bark (.png)
└── skybox/            px, nx, py, ny, pz, nz (.png)
```
