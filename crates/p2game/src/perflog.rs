//! Medidor de fps y log de frames (Android). Muestra fps, tiempo medio y peor frame en la parte
//! superior y escribe una línea por segundo en `game_log.txt`, en la carpeta de la app del
//! almacenamiento externo (Android/data/com.postal2.app/files), que el gestor de archivos puede abrir.
//! En escritorio no hace nada.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

#[cfg(target_os = "android")]
const LOG_PATH: &str = "/storage/emulated/0/Android/data/com.postal2.app/files/game_log.txt";

#[derive(Resource, Default)]
pub(crate) struct PerfState {
    spawned: bool,
    acc_ms: f32,
    frames: u32,
    total_ms: f32,
    worst_ms: f32,
    file: Option<std::fs::File>,
}

#[derive(Component)]
pub(crate) struct FpsText;

fn open_log(st: &mut PerfState) {
    #[cfg(target_os = "android")]
    {
        if let Some(dir) = std::path::Path::new(LOG_PATH).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(LOG_PATH) {
            let _ = writeln!(f, "--- sesión {} ---", unix_secs());
            st.file = Some(f);
        }
    }
    let _ = st;
}

fn unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub(crate) fn perf_system(
    mut commands: Commands,
    time: Res<Time>,
    mut st: ResMut<PerfState>,
    mut texts: Query<&mut Text, With<FpsText>>,
    overlay: Query<&Visibility, With<crate::touchctl::OverlayPart>>,
    meshes: Query<&Visibility, bevy::ecs::query::Or<(bevy::prelude::With<Mesh3d>, bevy::prelude::With<Mesh2d>)>>,
    touch: Res<crate::touchctl::TouchCtl>,
) {
    if !cfg!(target_os = "android") {
        return;
    }
    if !st.spawned {
        st.spawned = true;
        open_log(&mut st);
    }
    if texts.is_empty() {
        commands.spawn((
            FpsText,
            Text::new("-- fps"),
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(40.0),
                top: Val::Px(4.0),
                ..default()
            },
            GlobalZIndex(1001),
        ));
    }

    let dt_ms = time.delta_secs() * 1000.0;
    st.frames += 1;
    st.acc_ms += dt_ms;
    st.total_ms += dt_ms;
    st.worst_ms = st.worst_ms.max(dt_ms);

    if st.acc_ms >= 1000.0 {
        let fps = st.frames as f32 * 1000.0 / st.acc_ms;
        let avg = st.total_ms / st.frames as f32;
        let worst = st.worst_ms;
        for mut t in &mut texts {
            *t = Text::new(format!("{fps:.0} fps  {avg:.1} ms  peor {worst:.0} ms"));
        }
        // Nodos del overlay táctil: visibles/total (para saber si se dibujan).
        let total = overlay.iter().count();
        let visible = overlay.iter().filter(|v| matches!(v, Visibility::Visible)).count();
        // Mallas 3D/2D visibles y totales: aproximan los draw calls del frame.
        let mesh_total = meshes.iter().count();
        let mesh_vis = meshes.iter().filter(|v| !matches!(v, Visibility::Hidden)).count();
        let line = format!(
            "{} fps={fps:.1} media_ms={avg:.2} peor_ms={worst:.1} frames={} overlay={visible}/{total} mallas={mesh_vis}/{mesh_total} menu={}\n",
            unix_secs(),
            st.frames,
            touch.menu
        );
        if let Some(f) = st.file.as_mut() {
            let _ = f.write_all(line.as_bytes());
        }
        st.acc_ms = 0.0;
        st.frames = 0;
        st.total_ms = 0.0;
        st.worst_ms = 0.0;
    }
}

/// Escribe en `game_log.txt` cualquier pánico con su traza. En Android el mensaje de pánico no
/// llega a logcat por la salida estándar, así que sin esto el crash no deja rastro.
pub(crate) fn install_panic_hook() {
    #[cfg(target_os = "android")]
    {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let bt = std::backtrace::Backtrace::force_capture();
            append_log(&format!("PANIC {} {info}\n{bt}\n", unix_secs()));
            previous(info);
        }));
    }
}

#[cfg(target_os = "android")]
fn append_log(text: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(LOG_PATH) {
        let _ = f.write_all(text.as_bytes());
    }
}

/// Línea de informe (reparto de cada frame, distribución de tiempos). En escritorio va a la salida
/// estándar; en Android también a `game_log.txt`, con la hora.
pub(crate) fn report(line: &str) {
    println!("{line}");
    #[cfg(target_os = "android")]
    append_log(&format!("{} {line}\n", unix_secs()));
}

/// Ajustes de render para Android: antialiasing (MSAA) apagado en las cámaras 3D. Bevy lo activa
/// por defecto en 4x; en la GPU del teléfono es de lo que más ancho de banda consume.
pub(crate) fn render_tune(mut commands: Commands, cams: Query<Entity, Added<Camera3d>>) {
    if !cfg!(target_os = "android") {
        return;
    }
    for e in &cams {
        commands.entity(e).insert(Msaa::Off);
    }
}
