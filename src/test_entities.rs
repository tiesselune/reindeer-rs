use serde_derive::{Deserialize, Serialize};

use crate::Entity;

#[derive(Serialize, Deserialize)]
pub struct Entity1 {
    id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Entity2 {
    id: String,
}

#[derive(Serialize, Deserialize)]
pub struct Entity3 {
    id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Entity4 {
    id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ChildEntity1 {
    id: (String, u32),
}

#[derive(Serialize, Deserialize)]
pub struct ChildEntity2 {
    id: (u32, u32),
}

impl Entity for Entity1 {
    type Key = u32;

    fn tree_name() -> &'static str {
        "entity1"
    }

    fn get_key(&self) -> Self::Key {
        self.id
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.id = *key;
    }
}

impl Entity for Entity2 {
    type Key = String;

    fn tree_name() -> &'static str {
        "entity2"
    }

    fn get_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.id = key.clone();
    }
}

impl Entity for Entity3 {
    type Key = u32;

    fn tree_name() -> &'static str {
        "entity3"
    }

    fn get_key(&self) -> Self::Key {
        self.id
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.id = *key;
    }
}

impl Entity for Entity4 {
    type Key = u32;

    fn tree_name() -> &'static str {
        "entity4"
    }

    fn get_key(&self) -> Self::Key {
        self.id
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.id = *key
    }
}

impl Entity for ChildEntity1 {
    type Key = (String, u32);

    fn tree_name() -> &'static str {
        "child_entity_1"
    }

    fn get_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.id = key.clone();
    }
}

impl Entity for ChildEntity2 {
    type Key = (u32, u32);

    fn tree_name() -> &'static str {
        "child_entity_2"
    }

    fn get_key(&self) -> Self::Key {
        self.id
    }

    fn set_key(&mut self, key: &Self::Key) {
        self.id = *key;
    }
}
