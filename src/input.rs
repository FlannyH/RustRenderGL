use std::collections::HashMap;

use glfw::{Key, Action};

pub struct UserInput {
    key_state: HashMap<i32, bool>
}

impl UserInput {
    pub fn process_event(&mut self, event: &glfw::WindowEvent) {
        if let glfw::WindowEvent::Key(key, _, action, _) = event {
            self.key_state.insert(*key as i32, match action {
                Action::Press => true,
                Action::Release => false,
                Action::Repeat => true,
            });
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        if self.key_state.contains_key(&(key as i32)) {
        self.key_state[&(key as i32)]} else {false}
    }

    pub fn new() -> Self {
        UserInput { key_state: HashMap::new() }
    }
}