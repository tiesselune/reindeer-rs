use hashers::fx_hash::FxHasher;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use crate::entity::AsBytes;
use crate::Entity;

use super::DeletionBehaviour;

type RelationMap = HashMap<String, Vec<(Vec<u8>,DeletionBehaviour)>, BuildHasherDefault<FxHasher>>;

#[derive(Serialize, Deserialize,Default)]
pub struct RelationDescriptor {
    pub related_entities: RelationMap,
}

impl RelationDescriptor {
    pub fn add_related<E: Entity>(&mut self, e: &E, behaviour : DeletionBehaviour) {
        let key = e.get_key().as_bytes();
        if let Some(v) = self.related_entities.get_mut(E::tree_name()) {
            v.push((key,behaviour))
        } else {
            self.related_entities
                .insert(String::from(E::tree_name()), vec![(key,behaviour)]);
        }
    }

    pub fn remove_related_by_key<E: Entity>(&mut self, e: &[u8]) {
        self.remove_related_by_key_and_tree_name(E::tree_name(), e)
    }

    pub fn remove_related_by_key_and_tree_name(&mut self, tree: &str, e: &[u8]) {
        if let Some(v) = self.related_entities.get_mut(tree) {
            if let Some(index) = v
                .iter()
                .position(|(value,_)| value.to_ascii_lowercase() == e.to_ascii_lowercase())
            {
                v.remove(index);
            }
        }
    }

    pub fn empty(&mut self) {
        self.related_entities = HashMap::default();
    }
}
