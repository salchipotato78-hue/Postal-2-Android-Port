# Postal 2 en Android: estado del port

## Lo que incluye este cambio

- `android/`: proyecto Gradle/Kotlin con la app "Postal 2".
  - `MainActivity`: elige el ZIP con tus datos del juego (Storage Access Framework), lo extrae en el almacenamiento privado de la app, busca la carpeta con `System/`, `Maps/` y `Textures/` y guarda su ruta en `game_root.txt`. Tiene protección contra rutas maliciosas dentro del ZIP.
  - `GameActivity`: actividad nativa que carga `libpostal2.so`.
- `crates/p2touch/`: lógica de controles táctiles sin dependencias (probada con `cargo test -p p2touch`):
  - diseño editable: posición, tamaño, opacidad y visibilidad de cada botón;
  - botones: fuego, fuego alternativo, recargar, saltar, agacharse, usar, pausa, guardar arma, patear, arma siguiente y anterior;
  - un único joystick virtual para moverse; la cámara se mueve arrastrando en la mitad derecha, sin stick;
  - formato de archivo de diseño de texto (`to_text` / `from_text`);
  - `projection_aspect`: ultrawide sin estirar, acotado entre 4:3 y 21:9.
- `crates/p2android/`: punto de entrada de Android. Compila `libpostal2.so` (cdylib). Lee la carpeta del juego de `game_root.txt` (escrito por la app al importar el ZIP), replica el arranque de la ventana de escritorio y reutiliza los módulos de ventana de `p2game/src/main.rs` mediante `#[path]`, sin modificar el código de escritorio. El punto de entrada usa `#[bevy_main]`.
- `.github/workflows/android.yml`: compila `libpostal2.so` para arm64 con `cargo-ndk` y genera el APK de depuración. Sin `--locked`, porque el `Cargo.lock` no incluye los crates nuevos hasta que se regenere.
- `Cargo.toml`: añade `crates/p2touch` y `crates/p2android` al workspace.

## Pendiente (no está hecho ni probado)

1. **Verificar la compilación de `p2android`.** Se escribió sin compilador. Puntos a revisar primero: que `#[bevy_main]` funcione en una `cdylib` (en lugar de un binario), la versión de `android-activity` que usa Bevy 0.19, y las dependencias de ventana de Linux (x11/wayland de `default_platform`) en el target Android. También puede haber llamadas a la API de Windows sin `cfg` en `window_memory.rs`. Ejecuta `cargo ndk` localmente o en el workflow y arregla los errores que salgan.
2. **Regenerar `Cargo.lock`** (`cargo update -p p2android` o un `cargo build` normal) y luego volver a poner `--locked` en el workflow.
3. **Integración de `p2touch` con Bevy:** dibujar los botones como overlay, enviar los toques a `p2input` y el arrastre a la cámara, y el editor de diseño dentro del juego (menú de ajustes táctiles).
4. **Ultrawide:** conectar `projection_aspect` a la cámara de `p2view`.
5. **Menús y pausa táctiles** en la UI de `p2canvas`.
6. **Pruebas reales** en el Poco X8 Pro Max (GPU Mali). Mali con Vulkan es el primer punto a verificar.
7. **Rendimiento:** el ZIP de datos de POSTAL 2 pesa varios GB; la extracción puede tardar y no tiene barra de progreso todavía.

No se pudo compilar nada en este entorno (sin toolchain de Rust ni NDK), así que el código Kotlin, `p2android` y el workflow están sin probar.
