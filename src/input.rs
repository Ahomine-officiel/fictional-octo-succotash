//! Input layer: keyboard/mouse (P1) + P2 (manette OU clavier flèches+cluster
//! droit), UI navigation. KeyCode is physical, so WASD works on AZERTY as ZQSD
//! automatically, and the P2 keys are chosen with labels stable across layouts
//! (U O P J K L H Y + flèches).

use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Default)]
pub struct PlayerInput {
    pub mv: glam::Vec2,
    pub melee: bool,
    pub ranged: bool,
    pub roll: bool,
    pub artifacts: [bool; 3],
    pub potion: bool,
    pub interact: bool,
}

#[derive(Clone, Copy, Default)]
pub struct UINav {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub cancel: bool,
}

/// État d'UNE manette (indexée par son id gilrs) — chaque manette pilote son
/// propre joueur au lieu de tout fusionner.
#[derive(Default)]
pub struct PadState {
    pub mv: glam::Vec2,
    buttons: std::collections::HashSet<u16>,
    pressed: std::collections::HashSet<u16>,
}

impl PadState {
    fn snapshot(&self) -> PlayerInput {
        PlayerInput {
            mv: self.mv,
            melee: self.buttons.contains(&GP_SOUTH),
            ranged: self.buttons.contains(&GP_EAST),
            roll: self.buttons.contains(&GP_RB),
            artifacts: [
                self.pressed.contains(&GP_WEST),  // X
                self.pressed.contains(&GP_NORTH), // Y
                self.pressed.contains(&GP_LT2),   // L2
            ],
            potion: self.pressed.contains(&GP_LB),
            interact: self.pressed.contains(&GP_DPAD_UP),
        }
    }
}

#[derive(Default)]
pub struct Input {
    keys: std::collections::HashSet<KeyCode>,
    mouse: [bool; 2],
    /// edge-triggered keys this frame
    pressed: std::collections::HashSet<KeyCode>,
    mouse_pressed: [bool; 2],
    pub mouse_pos: (f32, f32),
    pub gamepad_count: usize,
    /// état PAR manette (index = id gilrs) — manette 0 => P2
    pub pads: Vec<PadState>,
    // état fusionné de toutes les manettes (menus / rétrocompatibilité)
    pad_move: glam::Vec2,
    pad_buttons: std::collections::HashSet<u16>,
    pad_pressed: std::collections::HashSet<u16>,
}

// gamepad button codes (gilrs Button as u16)
pub const GP_SOUTH: u16 = 0; // A / Croix
pub const GP_EAST: u16 = 1; // B / Cercle
pub const GP_NORTH: u16 = 3; // Y / Triangle
pub const GP_WEST: u16 = 2; // X / Carré
pub const GP_LB: u16 = 4;
pub const GP_RB: u16 = 5;
pub const GP_LT2: u16 = 6;
pub const GP_RT2: u16 = 7;
pub const GP_SELECT: u16 = 8;
pub const GP_START: u16 = 9;
pub const GP_DPAD_UP: u16 = 11;
pub const GP_DPAD_DOWN: u16 = 12;
pub const GP_DPAD_LEFT: u16 = 13;
pub const GP_DPAD_RIGHT: u16 = 14;

impl Input {
    pub fn new() -> Input {
        Input::default()
    }

    pub fn key(&self, k: KeyCode) -> bool {
        self.keys.contains(&k)
    }
    pub fn key_pressed(&self, k: KeyCode) -> bool {
        self.pressed.contains(&k)
    }
    pub fn mouse_down(&self, b: usize) -> bool {
        self.mouse[b]
    }
    pub fn mouse_pressed(&self, b: usize) -> bool {
        self.mouse_pressed[b]
    }

    pub fn on_key(&mut self, state: ElementState, k: KeyCode) {
        match state {
            ElementState::Pressed => {
                self.keys.insert(k);
                self.pressed.insert(k);
            }
            ElementState::Released => {
                self.keys.remove(&k);
            }
        }
    }

    pub fn on_mouse(&mut self, state: ElementState, b: MouseButton) {
        let i = match b {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            _ => return,
        };
        match state {
            ElementState::Pressed => {
                self.mouse[i] = true;
                self.mouse_pressed[i] = true;
            }
            ElementState::Released => self.mouse[i] = false,
        }
    }

    pub fn on_gamepad_button(&mut self, state: ElementState, code: u16) {
        match state {
            ElementState::Pressed => {
                self.pad_buttons.insert(code);
                self.pad_pressed.insert(code);
            }
            ElementState::Released => {
                self.pad_buttons.remove(&code);
            }
        }
    }

    pub fn on_gamepad_axis(&mut self, axis: u16, value: f32) {
        const DEAD: f32 = 0.22;
        let v = if value.abs() < DEAD { 0.0 } else { value };
        match axis {
            0 => self.pad_move.x = v,  // LeftStickX
            1 => self.pad_move.y = -v, // LeftStickY (screen-up positive)
            _ => {}
        }
    }

    pub fn end_frame(&mut self) {
        self.pressed.clear();
        self.mouse_pressed = [false, false];
        self.pad_pressed.clear();
        for p in &mut self.pads {
            p.pressed.clear();
        }
    }

    /// P1: keyboard + mouse. Quand le P2 clavier est actif, les flèches lui
    /// appartiennent (déplacement J2) et ne pilotent plus P1.
    pub fn p1_input(&self, p2_keyboard: bool) -> PlayerInput {
        let mut mv = glam::Vec2::ZERO;
        if !p2_keyboard && self.key(KeyCode::ArrowUp) {
            mv.y -= 1.0;
        }
        if !p2_keyboard && self.key(KeyCode::ArrowDown) {
            mv.y += 1.0;
        }
        if !p2_keyboard && self.key(KeyCode::ArrowLeft) {
            mv.x -= 1.0;
        }
        if !p2_keyboard && self.key(KeyCode::ArrowRight) {
            mv.x += 1.0;
        }
        if self.key(KeyCode::KeyW) {
            mv.y -= 1.0;
        }
        if self.key(KeyCode::KeyS) {
            mv.y += 1.0;
        }
        if self.key(KeyCode::KeyA) {
            mv.x -= 1.0;
        }
        if self.key(KeyCode::KeyD) {
            mv.x += 1.0;
        }
        PlayerInput {
            mv: mv.normalize_or_zero(),
            melee: self.mouse_down(0),
            ranged: self.mouse_down(1),
            roll: self.key(KeyCode::Space),
            artifacts: [
                self.key_pressed(KeyCode::Digit1),
                self.key_pressed(KeyCode::Digit2),
                self.key_pressed(KeyCode::Digit3),
            ],
            potion: self.key_pressed(KeyCode::KeyF),
            interact: self.key_pressed(KeyCode::KeyE) || self.key_pressed(KeyCode::KeyX),
        }
    }

    /// P2 clavier : flèches = déplacement, cluster droit = actions.
    ///Touches à libellé stable AZERTY/QWERTY (U O P J K L H Y).
    fn p2kb_input(&self) -> PlayerInput {
        let mut mv = glam::Vec2::ZERO;
        if self.key(KeyCode::ArrowUp) {
            mv.y -= 1.0;
        }
        if self.key(KeyCode::ArrowDown) {
            mv.y += 1.0;
        }
        if self.key(KeyCode::ArrowLeft) {
            mv.x -= 1.0;
        }
        if self.key(KeyCode::ArrowRight) {
            mv.x += 1.0;
        }
        PlayerInput {
            mv: mv.normalize_or_zero(),
            melee: self.key(KeyCode::KeyU),
            ranged: self.key(KeyCode::KeyO),
            roll: self.key(KeyCode::KeyP),
            artifacts: [
                self.key_pressed(KeyCode::KeyJ),
                self.key_pressed(KeyCode::KeyK),
                self.key_pressed(KeyCode::KeyL),
            ],
            potion: self.key_pressed(KeyCode::KeyH),
            interact: self.key_pressed(KeyCode::KeyY),
        }
    }

    /// P2: première manette disponible, sinon clavier (flèches + U/O/P...).
    pub fn p2_input(&self) -> PlayerInput {
        if let Some(pad) = self.pads.first() {
            return pad.snapshot();
        }
        self.p2kb_input()
    }

    /// Le P2 a-t-il une manette ? (pour les chips HUD adaptées)
    pub fn p2_on_pad(&self) -> bool {
        !self.pads.is_empty()
    }

    pub fn ui_nav(&self) -> UINav {
        let mut nav = UINav {
            up: self.key_pressed(KeyCode::ArrowUp) || self.key_pressed(KeyCode::KeyW),
            down: self.key_pressed(KeyCode::ArrowDown) || self.key_pressed(KeyCode::KeyS),
            left: self.key_pressed(KeyCode::ArrowLeft) || self.key_pressed(KeyCode::KeyA),
            right: self.key_pressed(KeyCode::ArrowRight) || self.key_pressed(KeyCode::KeyD),
            confirm: self.key_pressed(KeyCode::Enter) || self.key_pressed(KeyCode::Space) || self.mouse_pressed(0),
            cancel: self.key_pressed(KeyCode::Escape) || self.key_pressed(KeyCode::Backspace),
        };
        // merge gamepad
        nav.up |= self.pad_pressed.contains(&GP_DPAD_UP);
        nav.down |= self.pad_pressed.contains(&GP_DPAD_DOWN);
        nav.left |= self.pad_pressed.contains(&GP_DPAD_LEFT);
        nav.right |= self.pad_pressed.contains(&GP_DPAD_RIGHT);
        nav.confirm |= self.pad_pressed.contains(&GP_SOUTH);
        nav.cancel |= self.pad_pressed.contains(&GP_EAST) || self.pad_pressed.contains(&GP_START);
        nav
    }

    pub fn poll_gamepads(&mut self, gilrs: &mut Option<gilrs::Gilrs>) {
        if let Some(g) = gilrs {
            while let Some(gilrs::Event { id, event, .. }) = g.next_event() {
                let map_btn = |b: gilrs::Button| -> Option<u16> {
                    Some(match b {
                        gilrs::Button::South => GP_SOUTH,
                        gilrs::Button::East => GP_EAST,
                        gilrs::Button::North => GP_NORTH,
                        gilrs::Button::West => GP_WEST,
                        gilrs::Button::LeftTrigger => GP_LB,
                        gilrs::Button::RightTrigger => GP_RB,
                        gilrs::Button::LeftTrigger2 => GP_LT2,
                        gilrs::Button::RightTrigger2 => GP_RT2,
                        gilrs::Button::Select => GP_SELECT,
                        gilrs::Button::Start => GP_START,
                        gilrs::Button::DPadUp => GP_DPAD_UP,
                        gilrs::Button::DPadDown => GP_DPAD_DOWN,
                        gilrs::Button::DPadLeft => GP_DPAD_LEFT,
                        gilrs::Button::DPadRight => GP_DPAD_RIGHT,
                        _ => return None,
                    })
                };
                // route per-pad (pads[0] = P2), plus agrégat pour les menus
                let pi: usize = id.into();
                if self.pads.len() <= pi {
                    self.pads.resize_with(pi + 1, PadState::default);
                }
                match event {
                    gilrs::EventType::ButtonPressed(b, _code) => {
                        if let Some(c) = map_btn(b) {
                            self.on_gamepad_button(ElementState::Pressed, c);
                            self.pads[pi].pressed.insert(c);
                            self.pads[pi].buttons.insert(c);
                        }
                    }
                    gilrs::EventType::ButtonReleased(b, _code) => {
                        if let Some(c) = map_btn(b) {
                            self.on_gamepad_button(ElementState::Released, c);
                            self.pads[pi].buttons.remove(&c);
                        }
                    }
                    gilrs::EventType::AxisChanged(axis, value, _code) => {
                        let a = match axis {
                            gilrs::Axis::LeftStickX => 0u16,
                            gilrs::Axis::LeftStickY => 1u16,
                            _ => 99,
                        };
                        if a != 99 {
                            self.on_gamepad_axis(a, value);
                            const DEAD: f32 = 0.22;
                            let v = if value.abs() < DEAD { 0.0 } else { value };
                            if a == 0 {
                                self.pads[pi].mv.x = v;
                            } else {
                                self.pads[pi].mv.y = -v;
                            }
                        }
                    }
                    gilrs::EventType::Connected => {
                        self.gamepad_count += 1;
                    }
                    gilrs::EventType::Disconnected => {
                        self.gamepad_count = self.gamepad_count.saturating_sub(1);
                        if let Some(p) = self.pads.get_mut(pi) {
                            p.mv = glam::Vec2::ZERO;
                            p.buttons.clear();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
