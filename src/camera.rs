use glfw::Key;

use crate::{structs::Transform, input::UserInput};

pub struct Camera {
    pub transform: Transform,
    pub move_speed: f32,
    pub mouse_sensitivity: f32,
    mouse_pos_old: (f32, f32),
    should_skip_mouse_update: bool,
    pub pitch: f32,
    pub yaw: f32,
}

impl Camera {
    pub fn new(
        transform: Transform,
        move_speed: f32,
        mouse_sensitivity: f32,
    ) -> Self {
        Camera {
            transform,
            move_speed,
            mouse_sensitivity,
            mouse_pos_old: (0.0, 0.0),
            pitch: 0.0,
            yaw: 0.0,
            should_skip_mouse_update: true,
        }
    }
    pub fn update(&mut self, input: &UserInput, delta_time: f32) {
        // Moving forwards, backwards, left and right
        if input.is_key_down(Key::A) {
            self.transform.translation -= self.move_speed * delta_time * self.transform.right()
        }
        if input.is_key_down(Key::D) {
            self.transform.translation += self.move_speed * delta_time * self.transform.right()
        }
        if input.is_key_down(Key::W) {
            self.transform.translation += self.move_speed * delta_time * self.transform.forward()
        }
        if input.is_key_down(Key::S) {
            self.transform.translation -= self.move_speed * delta_time * self.transform.forward()
        }

        // Moving up and down, Minecraft style
        if input.is_key_down(Key::Space) {
            self.transform.translation += self.move_speed * delta_time * glam::vec3(0.0, 1.0, 0.0);
        }
        if input.is_key_down(Key::LeftShift) {
            self.transform.translation -= self.move_speed * delta_time * glam::vec3(0.0, 1.0, 0.0);
        }

/*
        // Movement speed increase, like in Minecraft spectator mode
        if let Some(result) = input.get_scroll_wheel() {
            let (_x, y) = result;
            self.move_speed *= 1.005_f32.powf(y);
        }

        // Mouse rotation
        if input.get_mouse_down(MouseButton::Right) {
            // Update mouse position
            let mouse_pos = input.get_mouse_pos(MouseMode::Pass).unwrap();
            let delta_mouse = (
                mouse_pos.0 - self.mouse_pos_old.0,
                mouse_pos.1 - self.mouse_pos_old.1,
            );
            self.mouse_pos_old = mouse_pos;

            // If the mouse position is a specific high value, that means we're still settling in after starting to hold right click
            if !self.should_skip_mouse_update {
                self.pitch -= delta_mouse.1 * self.mouse_sensitivity;
                self.pitch = self.pitch.clamp(-PI * 0.4999, PI * 0.4999);
                self.yaw -= delta_mouse.0 * self.mouse_sensitivity;
                self.transform.rotation =
                    glam::Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0)
            } else {
                self.should_skip_mouse_update = false;
            }
        } else {
            self.should_skip_mouse_update = true;
        }
        */
    }
}
