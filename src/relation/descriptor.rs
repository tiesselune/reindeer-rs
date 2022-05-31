use serde_derive::{Serialize,Deserialize};
use std::collections::{HashSet,HashMap};
use std::hash::BuildHasherDefault;
use hashers::fx_hash::FxHasher;

use crate::Entity;
use crate::entity::AsBytes;

#[derive(Serialize,Deserialize)]
pub struct RelationDescriptor {
    pub related_entities : HashMap<String,Vec<Vec<u8>>,BuildHasherDefault<FxHasher>>,
    pub children_trees : HashSet<String,BuildHasherDefault<FxHasher>>,
    pub sibling_trees : HashSet<String,BuildHasherDefault<FxHasher>>,
    pub parent_tree : Option<String>,
    pub cascade_deletion : bool,
}

impl RelationDescriptor {
    pub fn new() -> RelationDescriptor {
        RelationDescriptor { 
            related_entities: HashMap::default(), 
            children_trees: HashSet::default(), 
            sibling_trees: HashSet::default(),
            parent_tree: None,
            cascade_deletion : false,
        }
    }

    pub fn add_related<E : Entity>(&mut self, e : &E) {
        let key = e.get_key().as_bytes();
        if let Some(v) = self.related_entities.get_mut(E::tree_name()){
            v.push(key)
        }
        else {
            self.related_entities.insert(String::from(E::tree_name()), vec![key]);
        }
    }

    pub fn remove_related_by_key<E: Entity>(&mut self, e : &[u8]) {
        if let Some(v) = self.related_entities.get_mut(E::tree_name()){
            if let Some(index) = v
                .iter()
                .position(|value| value.to_ascii_lowercase() == e.to_ascii_lowercase())
            {
                v.remove(index);
            }
        }
    }

    pub fn add_child_tree<E : Entity>(&mut self) {
        self.children_trees.insert(String::from(E::tree_name()));
    }

    pub fn add_sibling_tree<E : Entity>(&mut self) {
        self.sibling_trees.insert(String::from(E::tree_name()));
    }

    pub fn empty(&mut self){
        self.related_entities = HashMap::default();
        self.children_trees = HashSet::default();
        self.sibling_trees = HashSet::default();

    }
}