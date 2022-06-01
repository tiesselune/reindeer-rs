use hashers::fx_hash::FxHasher;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use crate::entity::AsBytes;
use crate::Entity;

#[derive(Serialize, Deserialize)]
pub struct RelationDescriptor {
    pub related_entities: HashMap<String, Vec<Vec<u8>>, BuildHasherDefault<FxHasher>>,
}

impl RelationDescriptor {
    pub fn new() -> RelationDescriptor {
        RelationDescriptor {
            related_entities: HashMap::default(),
        }
    }

    pub fn add_related<E: Entity>(&mut self, e: &E) {
        let key = e.get_key().as_bytes();
        if let Some(v) = self.related_entities.get_mut(E::tree_name()) {
            v.push(key)
        } else {
            self.related_entities
                .insert(String::from(E::tree_name()), vec![key]);
        }
    }

    pub fn remove_related_by_key<E: Entity>(&mut self, e: &[u8]) {
        self.remove_related_by_key_and_tree_name(&E::tree_name(), e)
    }

    pub fn remove_related_by_key_and_tree_name(&mut self, tree: &str, e: &[u8]) {
        if let Some(v) = self.related_entities.get_mut(tree) {
            if let Some(index) = v
                .iter()
                .position(|value| value.to_ascii_lowercase() == e.to_ascii_lowercase())
            {
                v.remove(index);
            }
        }
    }

    pub fn empty(&mut self) {
        self.related_entities = HashMap::default();
    }
}
