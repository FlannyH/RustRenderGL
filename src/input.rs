use std::collections::HashMap;

use glfw::{Action, Key};

pub struct UserInput {
    key_state: HashMap<i32, bool>,
    mouse_button_state: HashMap<i32, bool>,
    mouse_pos: (f32, f32),
}

impl UserInput {
    pub fn process_event(&mut self, event: &glfw::WindowEvent) {
        // Handle key input
        if let glfw::WindowEvent::Key(key, _, action, _) = event {
            self.key_state.insert(
                *key as i32,
                match action {
                    Action::Press => true,
                    Action::Release => false,
                    Action::Repeat => true,
                },
            );
        }

        // Handle mouse buttons
        if let glfw::WindowEvent::MouseButton(button, action, _) = event {
            self.mouse_button_state.insert(
                *button as i32,
                match action {
                    Action::Press => true,
                    Action::Release => false,
                    Action::Repeat => true,
                },
            );
        }

        // Handle mouse position
        if let glfw::WindowEvent::CursorPos(x, y) = event {
            self.mouse_pos = (*x as f32, *y as f32);
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        if self.key_state.contains_key(&(key as i32)) {
            self.key_state[&(key as i32)]
        } else {
            false
        }
    }

    pub fn new() -> Self {
        UserInput {
            key_state: HashMap::new(),
            mouse_button_state: HashMap::new(),
            mouse_pos: (0.0, 0.0),
        }
    }

    pub(crate) fn get_scroll_wheel(&self) -> f32 {
        0.0
    }

    pub(crate) fn get_mouse_pos(&self) -> (f32, f32) {
        self.mouse_pos
    }

    pub(crate) fn get_mouse_down(&self, button: glfw::MouseButton) -> bool {
        if self.mouse_button_state.contains_key(&(button as i32)) {
            self.mouse_button_state[&(button as i32)]
        } else {
            false
        }
    }
}
