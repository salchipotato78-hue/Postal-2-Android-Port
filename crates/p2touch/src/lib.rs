//! Controles táctiles de Postal 2 para Android.
//!
//! Sin dependencias y sin Bevy: el diseño es datos puros (posiciones normalizadas 0..1 de la pantalla),
//! así que se puede probar con `cargo test -p p2touch` y se integra después en la capa de input.
//!
//! Formato del archivo de diseño (una entrada por línea, `#` para comentarios):
//! ```text
//! stick  <x> <y> <size> <opacity> <visible 0|1>
//! look   <sensitivity>
//! button <action> <x> <y> <size> <opacity> <visible 0|1>
//! ```
//! `x`/`y` son el centro del control (0..1 del ancho/alto), `size` es el diámetro como fracción del
//! lado menor de la pantalla, `opacity` de 0 a 1.

use std::fmt::Write as _;

/// Acciones del juego que puede disparar un botón. Los nombres coinciden con el archivo de diseño.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Fire,
    AltFire,
    Reload,
    Jump,
    Crouch,
    Use,
    Pause,
    Holster,
    Kick,
    NextWeapon,
    PrevWeapon,
}

impl Action {
    pub const ALL: [Action; 11] = [
        Action::Fire,
        Action::AltFire,
        Action::Reload,
        Action::Jump,
        Action::Crouch,
        Action::Use,
        Action::Pause,
        Action::Holster,
        Action::Kick,
        Action::NextWeapon,
        Action::PrevWeapon,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Action::Fire => "fire",
            Action::AltFire => "alt_fire",
            Action::Reload => "reload",
            Action::Jump => "jump",
            Action::Crouch => "crouch",
            Action::Use => "use",
            Action::Pause => "pause",
            Action::Holster => "holster",
            Action::Kick => "kick",
            Action::NextWeapon => "next_weapon",
            Action::PrevWeapon => "prev_weapon",
        }
    }

    /// Nombre del comando del juego. Los bindings del jugador deciden qué tecla lo dispara.
    pub fn command(self) -> &'static str {
        match self {
            Action::Fire => "Fire",
            Action::AltFire => "AltFire",
            Action::Reload => "Reload",
            Action::Jump => "Jump",
            Action::Crouch => "Duck",
            Action::Use => "Use",
            Action::Pause => "Pause",
            Action::Holster => "Holster",
            Action::Kick => "Kick",
            Action::NextWeapon => "NextWeapon",
            Action::PrevWeapon => "PrevWeapon",
        }
    }

    pub fn parse(s: &str) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| a.name() == s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub action: Action,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub opacity: f32,
    pub visible: bool,
}

/// Joystick virtual para moverse (el único stick; la cámara va por arrastre, sin stick de cámara).
#[derive(Debug, Clone, PartialEq)]
pub struct Stick {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub opacity: f32,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub stick: Stick,
    /// Sensibilidad del arrastre de cámara en la mitad derecha de la pantalla.
    pub look_sensitivity: f32,
    pub buttons: Vec<Button>,
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

impl Default for Layout {
    /// Diseño de `Postal2-touch/layout.png` (coordenadas del centro y tamaño en fracción de la altura).
    /// Holster y PrevWeapon no tienen icono en el diseño: quedan ocultos por defecto.
    fn default() -> Self {
        let b = |action, x, y, size, visible| Button { action, x, y, size, opacity: 0.8, visible };
        Layout {
            stick: Stick { x: 0.106, y: 0.794, size: 0.235, opacity: 0.8, visible: true },
            look_sensitivity: 1.0,
            buttons: vec![
                b(Action::Fire, 0.869, 0.805, 0.20, true),
                b(Action::AltFire, 0.832, 0.923, 0.07, true),
                b(Action::Reload, 0.051, 0.601, 0.17, true),
                b(Action::Jump, 0.920, 0.695, 0.14, true),
                b(Action::Crouch, 0.934, 0.884, 0.125, true),
                b(Action::Use, 0.818, 0.704, 0.14, true),
                b(Action::Kick, 0.791, 0.843, 0.14, true),
                b(Action::NextWeapon, 0.973, 0.573, 0.11, true),
                b(Action::Pause, 0.964, 0.058, 0.075, true),
                b(Action::Holster, 0.60, 0.90, 0.08, false),
                b(Action::PrevWeapon, 0.55, 0.90, 0.08, false),
            ],
        }
    }
}

/// Cabecera del formato. La versión sirve para reescribir diseños viejos (ver `TouchCtl::load`).
pub const LAYOUT_HEADER: &str = "# Postal 2 touch layout v3";

impl Layout {
    pub fn button(&self, action: Action) -> Option<&Button> {
        self.buttons.iter().find(|b| b.action == action)
    }

    pub fn button_mut(&mut self, action: Action) -> Option<&mut Button> {
        self.buttons.iter_mut().find(|b| b.action == action)
    }

    /// Mueve un botón; las coordenadas se limitan a la pantalla.
    pub fn set_position(&mut self, action: Action, x: f32, y: f32) -> bool {
        match self.button_mut(action) {
            Some(b) => {
                b.x = clamp01(x);
                b.y = clamp01(y);
                true
            }
            None => false,
        }
    }

    pub fn set_size(&mut self, action: Action, size: f32) -> bool {
        match self.button_mut(action) {
            Some(b) => {
                b.size = size.clamp(0.04, 0.5);
                true
            }
            None => false,
        }
    }

    pub fn set_opacity(&mut self, action: Action, opacity: f32) -> bool {
        match self.button_mut(action) {
            Some(b) => {
                b.opacity = clamp01(opacity);
                true
            }
            None => false,
        }
    }

    pub fn set_visible(&mut self, action: Action, visible: bool) -> bool {
        match self.button_mut(action) {
            Some(b) => {
                b.visible = visible;
                true
            }
            None => false,
        }
    }

    /// Botón visible bajo el toque (px, py) en una pantalla de w×h píxeles. Si hay varios, gana el más cercano al centro.
    pub fn hit(&self, px: f32, py: f32, w: f32, h: f32) -> Option<Action> {
        let min_side = w.min(h);
        let mut best: Option<(Action, f32)> = None;
        for b in self.buttons.iter().filter(|b| b.visible) {
            let cx = b.x * w;
            let cy = b.y * h;
            let radius = b.size * min_side * 0.5;
            let d2 = (px - cx).powi(2) + (py - cy).powi(2);
            if d2 <= radius * radius && best.map_or(true, |(_, bd)| d2 < bd) {
                best = Some((b.action, d2));
            }
        }
        best.map(|(a, _)| a)
    }

    /// Ejes del stick virtual (-1..1) para un toque en `touch` con el centro del stick en `center`.
    /// Devuelve (0, 0) dentro de la zona muerta. Radio en píxeles.
    pub fn stick_axes(center: (f32, f32), touch: (f32, f32), radius_px: f32) -> (f32, f32) {
        const DEADZONE: f32 = 0.15;
        if radius_px <= 0.0 {
            return (0.0, 0.0);
        }
        let dx = (touch.0 - center.0) / radius_px;
        let dy = (touch.1 - center.1) / radius_px;
        let len = (dx * dx + dy * dy).sqrt();
        if len < DEADZONE {
            return (0.0, 0.0);
        }
        // Reescala para que el borde de la zona muerta valga 0 y el borde del stick valga 1.
        let scale = ((len.min(1.0) - DEADZONE) / (1.0 - DEADZONE)) / len;
        (dx * scale, dy * scale)
    }

    /// Mantiene cada control (círculo) completo dentro de la pantalla de w×h.
    /// El radio se mide en la unidad del lado menor, igual que el dibujo.
    pub fn fit(&mut self, w: f32, h: f32) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let min_side = w.min(h);
        let fit_center = |x: f32, y: f32, size: f32| {
            let rx = (size * min_side * 0.5 / w).min(0.5);
            let ry = (size * min_side * 0.5 / h).min(0.5);
            (x.clamp(rx, 1.0 - rx), y.clamp(ry, 1.0 - ry))
        };
        let (sx, sy) = fit_center(self.stick.x, self.stick.y, self.stick.size);
        self.stick.x = sx;
        self.stick.y = sy;
        for b in &mut self.buttons {
            let (x, y) = fit_center(b.x, b.y, b.size);
            b.x = x;
            b.y = y;
        }
    }

    /// Serializa el diseño en el formato de texto documentado arriba.
    pub fn to_text(&self) -> String {
        let mut out = format!("{LAYOUT_HEADER}\n");
        let s = &self.stick;
        let _ = writeln!(out, "stick {:.3} {:.3} {:.3} {:.3} {}", s.x, s.y, s.size, s.opacity, s.visible as u8);
        let _ = writeln!(out, "look {:.3}", self.look_sensitivity);
        for b in &self.buttons {
            let _ = writeln!(
                out,
                "button {} {:.3} {:.3} {:.3} {:.3} {}",
                b.action.name(),
                b.x,
                b.y,
                b.size,
                b.opacity,
                b.visible as u8
            );
        }
        out
    }

    /// Lee el formato de texto. Las líneas inválidas devuelven error con su número de línea.
    /// Las acciones que no aparecen en el archivo conservan su valor por defecto.
    pub fn from_text(text: &str) -> Result<Layout, String> {
        let mut layout = Layout::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = |msg: &str| format!("línea {}: {}", n + 1, msg);
            let parts: Vec<&str> = line.split_whitespace().collect();
            let num = |i: usize| -> Result<f32, String> {
                parts
                    .get(i)
                    .ok_or_else(|| at("faltan valores"))?
                    .parse::<f32>()
                    .map_err(|_| at("número no válido"))
            };
            let flag = |i: usize| -> Result<bool, String> {
                match parts.get(i).copied() {
                    Some("1") => Ok(true),
                    Some("0") => Ok(false),
                    _ => Err(at("visible debe ser 0 o 1")),
                }
            };
            match parts[0] {
                "stick" => {
                    layout.stick = Stick {
                        x: clamp01(num(1)?),
                        y: clamp01(num(2)?),
                        size: num(3)?.clamp(0.1, 0.6),
                        opacity: clamp01(num(4)?),
                        visible: flag(5)?,
                    };
                }
                "look" => layout.look_sensitivity = num(1)?.clamp(0.1, 5.0),
                "button" => {
                    let name = parts.get(1).ok_or_else(|| at("falta la acción"))?;
                    let action = Action::parse(name).ok_or_else(|| at("acción desconocida"))?;
                    let button = Button {
                        action,
                        x: clamp01(num(2)?),
                        y: clamp01(num(3)?),
                        size: num(4)?.clamp(0.04, 0.5),
                        opacity: clamp01(num(5)?),
                        visible: flag(6)?,
                    };
                    match layout.button_mut(action) {
                        Some(existing) => *existing = button,
                        None => layout.buttons.push(button),
                    }
                }
                _ => return Err(at("línea no reconocida")),
            }
        }
        Ok(layout)
    }
}

/// Relación de aspecto para la proyección de la cámara (ultrawide sin estirar).
///
/// Se usa la relación real de la pantalla acotada a [4:3, 21:9]: el juego amplía el campo horizontal
/// en vez de deformar la imagen. Por encima de 21:9 se recorta para no exagerar la periferia.
pub fn projection_aspect(screen_w: f32, screen_h: f32) -> f32 {
    if screen_h <= 0.0 || screen_w <= 0.0 {
        return 16.0 / 9.0;
    }
    (screen_w / screen_h).clamp(4.0 / 3.0, 21.0 / 9.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips_through_text() {
        let original = Layout::default();
        let parsed = Layout::from_text(&original.to_text()).expect("parse");
        assert_eq!(parsed.buttons.len(), original.buttons.len());
        assert_eq!(parsed.stick.visible, original.stick.visible);
        let fire = parsed.button(Action::Fire).unwrap();
        assert!((fire.x - 0.88).abs() < 1e-3);
    }

    #[test]
    fn bad_line_reports_line_number() {
        let err = Layout::from_text("button fire 0.5 0.5 0.1 0.5 1\nbutton nope 0 0 0.1 1 1").unwrap_err();
        assert!(err.contains("línea 2"), "{err}");
    }

    #[test]
    fn hit_finds_nearest_visible_button() {
        let mut l = Layout::default();
        let (w, h) = (2000.0, 1000.0);
        let fire = l.button(Action::Fire).unwrap().clone();
        assert_eq!(l.hit(fire.x * w, fire.y * h, w, h), Some(Action::Fire));
        l.set_visible(Action::Fire, false);
        assert_eq!(l.hit(fire.x * w, fire.y * h, w, h), None);
    }

    #[test]
    fn stick_deadzone_and_full_deflection() {
        assert_eq!(Layout::stick_axes((0.0, 0.0), (5.0, 0.0), 100.0), (0.0, 0.0));
        let (x, y) = Layout::stick_axes((0.0, 0.0), (0.0, 300.0), 100.0);
        assert!(x.abs() < 1e-6 && (y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ultrawide_is_clamped() {
        assert!((projection_aspect(3440.0, 1440.0) - 21.0 / 9.0).abs() < 1e-3);
        assert!((projection_aspect(1600.0, 1200.0) - 4.0 / 3.0).abs() < 1e-3);
    }
}
