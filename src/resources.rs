use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::Path,
};

use crate::{material::Material, mesh::Model, texture::Texture};

pub struct Resources {
    pub textures: HashMap<u64, Texture>,
    pub materials: HashMap<u64, Material>,
    pub models: HashMap<u64, Model>,
}

impl Resources {
    pub fn new() -> Self {
        Resources {
            textures: HashMap::new(),
            materials: HashMap::new(),
            models: HashMap::new(),
        }
    }

    pub fn load_model(&mut self, path: &Path) -> Result<u64, ()> {
        // Try to load model
        let model = Model::load_gltf(path);

        if let Ok(model_ok) = model {
            // Calculate hash for path
            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            let hash_id = hasher.finish();

            // Insert the model into the hashmap
            self.models.insert(hash_id, model_ok);
            return Ok(hash_id);
        }
        return Err(());
    }
}
