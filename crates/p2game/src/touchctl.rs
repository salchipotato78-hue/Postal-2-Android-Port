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
                Ok(text) => match Layout::from_text(&text) {
                    Ok(l) => layout = l,
                    Err(e) => eprintln!("touch_layout.txt: {e}; uso el diseño por defecto"),
                },
                Err(_) => {
                    let _ = std::fs::write(PATH, layout.to_text());
                }
            }
        }
        TouchCtl { layout, fingers: HashMap::new() }
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
    resolve: &mut dyn FnMut(&str) -> Option<Key>,
) -> TouchFrame {
    let (w, h) = size;
    let min_side = w.min(h).max(1.0);
    let layout = ctl.layout.clone();
    let mut out = TouchFrame { events: Vec::new(), mouse: None };

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
                    out.mouse = Some((p.x, p.y));
                } else {
                    let d = p - f.last;
                    if d != Vec2::ZERO {
                        let g = layout.look_sensitivity;
                        out.events.push(ev(Key::MOUSE_X, KeyAction::Axis, d.x * g));
                        out.events.push(ev(Key::MOUSE_Y, KeyAction::Axis, -d.y * g));
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
            out.mouse = Some((f.last.x, f.last.y));
            out.events.push(ev(Key::LEFT_MOUSE, KeyAction::Press, 0.0));
            out.events.push(ev(Key::LEFT_MOUSE, KeyAction::Release, 0.0));
        }
    }
    out
}

// --- On-screen controls (Android only) -------------------------------------------------------

/// Marks one overlay node: `None` is the stick, `Some` a button.
#[derive(Component)]
pub(crate) struct OverlayPart(Option<Action>);

#[derive(Resource, Default)]
pub(crate) struct TouchOverlay {
    spawned: bool,
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

/// Draws the stick and the buttons where the layout says, with its size and opacity.
pub(crate) fn overlay_system(
    mut commands: Commands,
    mut overlay: ResMut<TouchOverlay>,
    ctl: Res<TouchCtl>,
    mut parts: Query<(&OverlayPart, &mut Node, &mut BackgroundColor, &mut Visibility)>,
) {
    if !cfg!(target_os = "android") {
        return;
    }
    let l = &ctl.layout;
    if !overlay.spawned {
        overlay.spawned = true;
        let mut specs: Vec<(Option<Action>, &'static str)> = vec![(None, "")];
        specs.extend(l.buttons.iter().map(|b| (Some(b.action), label(b.action))));
        for (target, text) in specs {
            commands
                .spawn((
                    OverlayPart(target),
                    Node {
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderRadius::MAX,
                    BackgroundColor(Color::NONE),
                    Visibility::Hidden,
                ))
                .with_children(|p| {
                    if !text.is_empty() {
                        p.spawn((Text::new(text), TextColor(Color::WHITE)));
                    }
                });
        }
    }
    for (part, mut node, mut bg, mut vis) in &mut parts {
        let (x, y, size, opacity, visible, color) = match part.0 {
            None => (l.stick.x, l.stick.y, l.stick.size, l.stick.opacity, l.stick.visible, Color::srgb(0.7, 0.7, 0.7)),
            Some(a) => match l.button(a) {
                Some(b) => (b.x, b.y, b.size, b.opacity, b.visible, Color::srgb(0.9, 0.5, 0.2)),
                None => continue,
            },
        };
        node.left = Val::Percent(x * 100.0);
        node.top = Val::Percent(y * 100.0);
        node.width = Val::Vmin(size * 100.0);
        node.height = Val::Vmin(size * 100.0);
        node.margin = UiRect { left: Val::Vmin(-size * 50.0), top: Val::Vmin(-size * 50.0), ..default() };
        bg.0 = color.with_alpha(opacity);
        *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
    }
}
