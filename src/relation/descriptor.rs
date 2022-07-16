use hashers::fx_hash::FxHasher;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use crate::entity::AsBytes;
use crate::Entity;

use super::{DeletionBehaviour};

#[doc(hidden)]
pub type RelationMap =
    HashMap<String, Vec<RelationDescriptor>, BuildHasherDefault<FxHasher>>;

#[doc(hidden)]
#[derive(Serialize, Deserialize, Default)]
pub struct EntityRelations {
    pub related_entities: RelationMap,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize,PartialEq,Eq)]
pub struct RelationDescriptor {
    pub key : Vec<u8>,
    pub deletion_behaviour : DeletionBehaviour,
    pub name : Option<String>,
}

impl RelationDescriptor {
    fn new(key : &[u8], deletion_behaviour : DeletionBehaviour, name : Option<&str>) -> RelationDescriptor {
        RelationDescriptor { key : key.to_owned(), deletion_behaviour, name : name.map(|s| s.to_owned()) }
    }
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Default)]
pub struct FamilyDescriptor {
    pub tree_name: String,
    pub sibling_trees: Vec<(String, DeletionBehaviour)>,
    pub child_trees: Vec<(String, DeletionBehaviour)>,
}

#[doc(hidden)]
impl EntityRelations {
    pub fn add_related<E: Entity>(&mut self, e: &E, behaviour: DeletionBehaviour, name : Option<&str>) {
        let key = e.get_key().as_bytes();
        self.add_related_by_key(E::store_name(), &key, behaviour, name);
    }

    pub fn add_related_by_key(
        &mut self,
        tree_name: &str,
        key: &[u8],
        behaviour: DeletionBehaviour,
        name : Option<&str>,
    ) {
        if let Some(v) = self.related_entities.get_mut(tree_name) {
            let relation_descriptor = RelationDescriptor::new(key, behaviour,name);
            if !v.contains(&relation_descriptor) {
                v.push(relation_descriptor);
            }
            
        } else {
            self.related_entities
                .insert(String::from(tree_name), vec![RelationDescriptor::new(key, behaviour,name)]);
        }
    }

    pub fn remove_related_by_key<E: Entity>(&mut self, e: &[u8]) {
        self.remove_related_by_key_and_tree_name(E::store_name(), e)
    }

    pub fn remove_related_by_key_with_name<E: Entity>(&mut self, e: &[u8], name : &str) {
        self.remove_related_by_key_and_tree_name_with_name(E::store_name(), e, name)
    }

    pub fn remove_related_by_key_and_tree_name(&mut self, tree: &str, e: &[u8]) {
        if let Some(v) = self.related_entities.get_mut(tree) {
            v.retain(|rd| rd.key.to_ascii_lowercase() != e.to_ascii_lowercase());
        }
    }

    pub fn remove_related_by_key_and_tree_name_with_name(&mut self, tree: &str, e: &[u8], name : &str) {
        if let Some(v) = self.related_entities.get_mut(tree) {
            v.retain(|rd| rd.key.to_ascii_lowercase() != e.to_ascii_lowercase() && if let Some(r_name) = &rd.name {name == r_name} else { false });
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
