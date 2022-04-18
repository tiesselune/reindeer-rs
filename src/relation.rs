use serde_derive::{Deserialize, Serialize};
use sled::Db;

use crate::entity::{AsBytes, Entity};

#[derive(Serialize, Deserialize)]
pub struct RelationSingle<T> {
    #[serde(skip_serializing)]
    pub value: Option<T>,
    pub key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct RelationMany<T> {
    #[serde(skip_serializing)]
    pub values: Option<Vec<T>>,
    pub keys: Vec<Vec<u8>>,
}

impl<T: Entity> RelationSingle<T> {
    pub fn new(val: T) -> Self {
        let key = val.get_key().as_bytes();
        Self {
            value: Some(val),
            key,
        }
    }

    pub fn from_key(key: T::Key) -> Self {
        Self {
            value: None,
            key: key.as_bytes(),
        }
    }

    pub fn restore(&mut self, db: &Db) -> std::io::Result<()> {
        match &self.value {
            Some(_) => Ok(()),
            None => {
                self.value = T::get_from_u8_array(&self.key, db)?;
                Ok(())
            }
        }
    }
    pub fn remove(&mut self, db: &Db) -> std::io::Result<()> {
        self.value = None;
        T::remove_from_u8_array(&self.key, db)
    }
}

impl<T: Entity> RelationMany<T> {
    pub fn restore(&mut self, db: &Db) -> std::io::Result<()> {
        if let Some(_) = self.values {
            return Ok(());
        }
        let mut values = Vec::new();
        for v in &self.keys {
            let opt = T::get_from_u8_array(v, db)?;
            match opt {
                None => {}
                Some(val) => {
                    values.push(val);
                }
            }
        }
        self.values = Some(values);
        Ok(())
    }

    pub fn remove_all(&mut self, db: &Db) -> std::io::Result<()> {
        for v in &self.keys {
            T::remove_from_u8_array(v, db)?;
        }
        Ok(())
    }
}
