//! Android touch controls. Taps and drags become the same events the keyboard and mouse produce,
//! so the player's own bindings decide what each control does:
//! - Buttons press the key bound to their command (`Fire`, `Jump`, `Duck`, ...).
//! - The stick presses `MoveForward` / `MoveBack` / `StrafeLeft` / `StrafeRight`.
//! - Dragging on the right half moves the camera (mouse-look axis events).
//! - While a menu is up every tap is a click at its position (dragging does nothing).
//! On desktop there are no touches, so nothing here runs.

use std::collections::HashMap;

use bevy::input::touch::Touches;
use bevy::prelude::*;
use p2input::bevy_map::InputEvent;
use p2input::{Action as KeyAction, Key};
use p2touch::{Action, Layout};

/// Stick directions, by the game's own command names.
const STICK_COMMANDS: [&str; 4] = ["MoveForward", "MoveBack", "StrafeLeft", "StrafeRight"];
/// Stick deflection (0..1) before a direction counts as pressed.
const STICK_DEADZONE: f32 = 0.3;
/// A press that stays this close (px) and this short (s) is a tap.
const TAP_SLOP_PX: f32 = 16.0;
const TAP_MAX_SECS: f32 = 0.3;

#[derive(Clone, Copy)]
enum Zone {
    Button(Action),
    Stick,
    Look,
}

struct Finger {
    zone: Zone,
    start: Vec2,
    last: Vec2,
    age: f32,
    /// Key held by a button, released when the finger lifts.
    key: Option<Key>,
    /// Stick commands currently held, with the key each one resolved to.
    held: Vec<(&'static str, Key)>,
}

#[derive(Resource)]
pub(crate) struct TouchCtl {
    pub(crate) layout: Layout,
    fingers: HashMap<u64, Finger>,
    /// Stick deflection this frame (-1..1 each axis), for the knob. None when not touched.
    pub(crate) knob: Option<(f32, f32)>,
    /// True while a menu owns the input: the overlay is hidden then.
    pub(crate) menu: bool,
    /// Segundos hasta la próxima comprobación del archivo de diseño.
    reload_in: f32,
    /// Última fecha de modificación vista del archivo de diseño.
    mtime: Option<std::time::SystemTime>,
}

impl TouchCtl {
    /// On Android the layout lives in the app's files dir (`touch_layout.txt`); it is created with
    /// the defaults the first time. Elsewhere, or if the file is broken, the defaults are used.
    pub(crate) fn load() -> Self {
        let mut layout = Layout::default();
        #[cfg(target_os = "android")]
        {
            const PATH: &str = "/data/data/com.postal2.app/files/touch_layout.txt";
            match std::fs::read_to_string(PATH) {
                // Archivo de otra versión (diseño viejo): se reemplaza por el actual.
                Ok(text) if text.starts_with(p2touch::LAYOUT_HEADER) => match Layout::from_text(&text) {
                    Ok(l) => layout = l,
                    Err(e) => eprintln!("touch_layout.txt: {e}; uso el diseño por defecto"),
                },
                _ => {
                    let _ = std::fs::write(PATH, layout.to_text());
                }
            }
        }
        TouchCtl { layout, fingers: HashMap::new(), knob: None, menu: false, reload_in: 0.0, mtime: None }
    }
}

/// Archivo del diseño en Android (el editor lo escribe).
const LAYOUT_PATH: &str = "/data/data/com.postal2.app/files/touch_layout.txt";

impl TouchCtl {
    /// Vuelve a leer el diseño cuando el archivo cambia, para ver los cambios del editor sin reiniciar.
    /// Mira la fecha de modificación cada medio segundo; si el archivo no es de esta versión, lo ignora.
    fn reload_if_changed(&mut self, dt: f32) {
        if !cfg!(target_os = "android") {
            return;
        }
        self.reload_in -= dt;
        if self.reload_in > 0.0 {
            return;
        }
        self.reload_in = 0.5;
        let Ok(meta) = std::fs::metadata(LAYOUT_PATH) else { return };
        let Ok(modified) = meta.modified() else { return };
        if self.mtime == Some(modified) {
            return;
        }
        self.mtime = Some(modified);
        if let Ok(text) = std::fs::read_to_string(LAYOUT_PATH) {
            if text.starts_with(p2touch::LAYOUT_HEADER) {
                if let Ok(layout) = Layout::from_text(&text) {
                    self.layout = layout;
                }
            }
        }
    }
}

pub(crate) struct TouchFrame {
    pub(crate) events: Vec<InputEvent>,
    /// Cursor position to use this frame (menus follow the finger).
    pub(crate) mouse: Option<(f32, f32)>,
}

fn ev(key: Key, action: KeyAction, delta: f32) -> InputEvent {
    InputEvent { key, action, delta }
}

fn button_key(a: Action, resolve: &mut dyn FnMut(&str) -> Option<Key>) -> Option<Key> {
    if a == Action::Pause {
        Some(Key::ESCAPE)
    } else {
        resolve(a.command())
    }
}

/// Turns this frame's touches into input events. `size` is the window size in the same units as
/// the touch positions; `menu` is whether the UI currently owns the input.
pub(crate) fn frame(
    ctl: &mut TouchCtl,
    touches: &Touches,
    size: (f32, f32),
    menu: bool,
    dt: f32,
    scale: f32,
    resolve: &mut dyn FnMut(&str) -> Option<Key>,
) -> TouchFrame {
    let (w, h) = size;
    ctl.menu = menu;
    ctl.reload_if_changed(dt);
    ctl.layout.fit(w, h);
    // Positions are logical (points); the UI and the camera work in physical pixels.
    let min_side = w.min(h).max(1.0);
    let layout = ctl.layout.clone();
    let mut out = TouchFrame { events: Vec::new(), mouse: None };
    ctl.knob = None;

    // New fingers: which zone they start in.
    for t in touches.iter_just_pressed() {
        let p = t.position();
        let zone = if menu {
            Zone::Look
        } else if let Some(a) = layout.hit(p.x, p.y, w, h) {
            Zone::Button(a)
        } else if p.x < w * 0.5 {
            Zone::Stick
        } else {
            Zone::Look
        };
        let mut f = Finger { zone, start: p, last: p, age: 0.0, key: None, held: Vec::new() };
        if let Zone::Button(a) = zone {
            if let Some(k) = button_key(a, resolve) {
                out.events.push(ev(k, KeyAction::Press, 0.0));
                f.key = Some(k);
            }
        }
        ctl.fingers.insert(t.id(), f);
    }

    // Fingers still down: stick directions, camera drag, menu hover.
    for t in touches.iter() {
        let p = t.position();
        let Some(f) = ctl.fingers.get_mut(&t.id()) else { continue };
        f.age += dt;
        match f.zone {
            Zone::Look => {
                if menu {
                    out.mouse = Some((p.x * scale, p.y * scale));
                } else {
                    let d = p - f.last;
                    if d != Vec2::ZERO {
                        let g = layout.look_sensitivity;
                        out.events.push(ev(Key::MOUSE_X, KeyAction::Axis, d.x * g * scale));
                        out.events.push(ev(Key::MOUSE_Y, KeyAction::Axis, -d.y * g * scale));
                    }
                }
            }
            Zone::Stick => {
                let center = Vec2::new(layout.stick.x * w, layout.stick.y * h);
                let radius = layout.stick.size * min_side * 0.5;
                let (ax, ay) = if menu {
                    (0.0, 0.0)
                } else {
                    Layout::stick_axes((center.x, center.y), (p.x, p.y), radius)
                };
                let mut want: Vec<&'static str> = Vec::new();
                if ay < -STICK_DEADZONE {
                    want.push(STICK_COMMANDS[0]);
                }
                if ay > STICK_DEADZONE {
                    want.push(STICK_COMMANDS[1]);
                }
                if ax < -STICK_DEADZONE {
                    want.push(STICK_COMMANDS[2]);
                }
                if ax > STICK_DEADZONE {
                    want.push(STICK_COMMANDS[3]);
                }
                if !menu {
                    ctl.knob = Some((ax, ay));
                }
                f.held.retain(|(cmd, k)| {
                    if want.contains(cmd) {
                        true
                    } else {
                        out.events.push(ev(*k, KeyAction::Release, 0.0));
                        false
                    }
                });
                for cmd in want {
                    if !f.held.iter().any(|(c, _)| *c == cmd) {
                        if let Some(k) = resolve(cmd) {
                            out.events.push(ev(k, KeyAction::Press, 0.0));
                            f.held.push((cmd, k));
                        }
                    }
                }
            }
            Zone::Button(_) => {}
        }
        f.last = p;
    }

    // Lifted fingers: release what they held; a menu tap becomes a click.
    let released: Vec<u64> = touches.iter_just_released().map(|t| t.id()).collect();
    for id in released {
        let Some(f) = ctl.fingers.remove(&id) else { continue };
        if let Some(k) = f.key {
            out.events.push(ev(k, KeyAction::Release, 0.0));
        }
        for (_, k) in f.held {
            out.events.push(ev(k, KeyAction::Release, 0.0));
        }
        if menu && (f.last - f.start).length() <= TAP_SLOP_PX && f.age <= TAP_MAX_SECS {
            out.mouse = Some((f.last.x * scale, f.last.y * scale));
            out.events.push(ev(Key::LEFT_MOUSE, KeyAction::Press, 0.0));
            out.events.push(ev(Key::LEFT_MOUSE, KeyAction::Release, 0.0));
        }
    }
    out
}

// --- On-screen controls (Android only) -------------------------------------------------------

/// What one overlay node draws: the stick ring, its knob (follows the thumb) or a button.
#[derive(Component, Clone, Copy)]
pub(crate) enum OverlayPart {
    Stick,
    Knob,
    Button(Action),
}

#[derive(Resource, Default)]
pub(crate) struct TouchOverlay {
    spawned: bool,
}

/// Knob diameter as a fraction of the ring (the knob image is half the ring image).
const KNOB_RATIO: f32 = 0.5;

/// Icon of each control, from the design in assets/touch. None: no icon, drawn as a labelled dot.
fn icon(a: Action) -> Option<&'static str> {
    Some(match a {
        Action::Fire => "touch/disparar.png",
        Action::AltFire => "touch/Alt.png",
        Action::Reload => "touch/Recargar.png",
        Action::Jump => "touch/saltar.png",
        Action::Crouch => "touch/agacharse.png",
        Action::Use => "touch/usar.png",
        Action::Pause => "touch/pausa.png",
        Action::Kick => "touch/Patear.png",
        Action::NextWeapon => "touch/cambiar-arma.png",
        Action::Holster | Action::PrevWeapon => return None,
    })
}

fn label(a: Action) -> &'static str {
    match a {
        Action::Fire => "Disparar",
        Action::AltFire => "Alt",
        Action::Reload => "Recargar",
        Action::Jump => "Saltar",
        Action::Crouch => "Agachar",
        Action::Use => "Usar",
        Action::Pause => "Pausa",
        Action::Holster => "Guardar",
        Action::Kick => "Patear",
        Action::NextWeapon => "Sig.",
        Action::PrevWeapon => "Ant.",
    }
}

fn spawn_part(
    commands: &mut Commands,
    assets: &AssetServer,
    part: OverlayPart,
    icon: Option<&'static str>,
    text: &'static str,
) {
    // Los iconos son rombos: no llevan radio de borde, o se recortarían las esquinas.
    let node = Node {
        position_type: PositionType::Absolute,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    match icon {
        Some(path) => {
            commands.spawn((part, GlobalZIndex(1000), node, ImageNode::new(assets.load(path)), Visibility::Hidden));
        }
        None => {
            let dot = Node { border_radius: BorderRadius::MAX, ..node };
            commands
                .spawn((part, GlobalZIndex(1000), dot, BackgroundColor(Color::NONE), Visibility::Hidden))
                .with_children(|p| {
                    p.spawn((Text::new(text), TextColor(Color::WHITE)));
                });
        }
    }
}

/// Places a node centred at (x, y) of the screen, `size` of the height across, nudged by (dx, dy)
/// in VMin units (so the knob moves in the same units as the ring).
fn place(node: &mut Node, x: f32, y: f32, size: f32, dx: f32, dy: f32) {
    node.left = Val::Percent(x * 100.0);
    node.top = Val::Percent(y * 100.0);
    node.width = Val::VMin(size * 100.0);
    node.height = Val::VMin(size * 100.0);
    node.margin = UiRect {
        left: Val::VMin(-size * 50.0 + dx),
        top: Val::VMin(-size * 50.0 + dy),
        ..default()
    };
}

/// Draws the stick ring with its knob, and the buttons, where the layout says. Hidden in menus.
pub(crate) fn overlay_system(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut overlay: ResMut<TouchOverlay>,
    ctl: Res<TouchCtl>,
    mut parts: Query<(&OverlayPart, &mut Node, &mut Visibility, Option<&mut ImageNode>, Option<&mut BackgroundColor>)>,
) {
    if !cfg!(target_os = "android") {
        return;
    }
    let l = &ctl.layout;
    // Se crea (o se vuelve a crear) cuando no hay nodos: el juego borra sus entidades al cambiar de escena.
    if parts.is_empty() {
        overlay.spawned = true;
        eprintln!("touch overlay: {} piezas", 2 + l.buttons.len());
        spawn_part(&mut commands, &assets, OverlayPart::Stick, Some("touch/joystick-base.png"), "");
        spawn_part(&mut commands, &assets, OverlayPart::Knob, Some("touch/joystick-adentro.png"), "");
        for b in &l.buttons {
            spawn_part(&mut commands, &assets, OverlayPart::Button(b.action), icon(b.action), label(b.action));
        }
    }
    let s = &l.stick;
    // Knob offset: the thumb's deflection, scaled to the ring radius (stick.size * 50 VMin).
    let (ax, ay) = ctl.knob.unwrap_or((0.0, 0.0));
    for (part, mut node, mut vis, image, bg) in &mut parts {
        let (x, y, size, opacity, visible, dx, dy, tint) = match *part {
            OverlayPart::Stick => (s.x, s.y, s.size, s.opacity, s.visible, 0.0, 0.0, Color::srgb(0.9, 0.9, 0.9)),
            OverlayPart::Knob => {
                let k = s.size * KNOB_RATIO;
                (s.x, s.y, k, s.opacity, s.visible, ax * s.size * 50.0, ay * s.size * 50.0, Color::srgb(0.9, 0.9, 0.9))
            }
            OverlayPart::Button(a) => match l.button(a) {
                Some(b) => (b.x, b.y, b.size, b.opacity, b.visible, 0.0, 0.0, Color::srgb(0.9, 0.5, 0.2)),
                None => continue,
            },
        };
        place(&mut node, x, y, size, dx, dy);
        let show = visible && !ctl.menu;
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
        if let Some(mut img) = image {
            img.color = Color::srgba(1.0, 1.0, 1.0, opacity);
        }
        if let Some(mut bg) = bg {
            bg.0 = tint.with_alpha(opacity);
        }
    }
}
