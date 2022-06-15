use hashers::fx_hash::FxHasher;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use crate::entity::AsBytes;
use crate::Entity;

use super::DeletionBehaviour;

#[doc(hidden)]
pub type RelationMap =
    HashMap<String, Vec<(Vec<u8>, DeletionBehaviour)>, BuildHasherDefault<FxHasher>>;

#[doc(hidden)]
#[derive(Serialize, Deserialize, Default)]
pub struct RelationDescriptor {
    pub related_entities: RelationMap,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Default)]
pub struct FamilyDescriptor {
    pub tree_name: String,
    pub sibling_trees: Vec<(String, DeletionBehaviour)>,
    pub child_trees: Vec<(String, DeletionBehaviour)>,
}

#[doc(hidden)]
impl RelationDescriptor {
    pub fn add_related<E: Entity>(&mut self, e: &E, behaviour: DeletionBehaviour) {
        let key = e.get_key().as_bytes();
        self.add_related_by_key(E::store_name(), &key, behaviour);
    }

    pub fn add_related_by_key(
        &mut self,
        tree_name: &str,
        key: &[u8],
        behaviour: DeletionBehaviour,
    ) {
        if let Some(v) = self.related_entities.get_mut(tree_name) {
            v.push((key.to_vec(), behaviour))
        } else {
            self.related_entities
                .insert(String::from(tree_name), vec![(key.to_vec(), behaviour)]);
        }
    }

    pub fn remove_related_by_key<E: Entity>(&mut self, e: &[u8]) {
        self.remove_related_by_key_and_tree_name(E::store_name(), e)
    }

    pub fn remove_related_by_key_and_tree_name(&mut self, tree: &str, e: &[u8]) {
        if let Some(v) = self.related_entities.get_mut(tree) {
            if let Some(index) = v
                .iter()
                .position(|(value, _)| value.to_ascii_lowercase() == e.to_ascii_lowercase())
            {
                v.remove(index);
            }
        }
    }
}

#[doc(hidden)]
impl Entity for FamilyDescriptor {
    type Key = String;

    fn store_name() -> &'static str {
        "__$family_rel"
    }

    fn get_key(&self) -> &Self::Key {
        &self.tree_name
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.tree_name = key.clone();
    }
}
