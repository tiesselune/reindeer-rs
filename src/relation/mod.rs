mod descriptor;

use sled::Db;

use crate::entity::{AsBytes, Entity};

pub struct Relation;


impl Relation {
    pub fn create<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> std::io::Result<()> {
        Relation::create_link(e1, e2, db)?;
        Relation::create_link(e2, e1, db)?;
        Ok(())
    }

    pub fn remove<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> std::io::Result<()> {
        Relation::remove_link(e1, e2, db)?;
        Relation::remove_link(e2, e1, db)?;
        Ok(())
    }

    pub fn remove_by_keys<E1: Entity, E2: Entity>(
        e1: &[u8],
        e2: &[u8],
        db: &Db,
    ) -> std::io::Result<()> {
        Relation::remove_link_with_keys::<E1, E2>(e1, e2, db)?;
        Relation::remove_link_with_keys::<E1, E2>(e2, e1, db)?;
        Ok(())
    }

    fn tree_name<E1: Entity, E2: Entity>() -> String {
        format!("__rel_{}_{}", E1::tree_name(), E2::tree_name())
    }

    pub fn referers<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> std::io::Result<Vec<Vec<u8>>> {
        let tree = db.open_tree(&Relation::tree_name::<E1, E2>())?;
        if let Some(vec) = tree.get(e1.get_key().as_bytes())? {
            let key_list: Vec<Vec<u8>> = bincode::deserialize(vec.as_ref()).unwrap();
            Ok(key_list)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn has_referers<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> bool {
        if let Ok(v) = Relation::referers::<E1, E2>(e1, db) {
            v.len() > 0
        } else {
            false
        }
    }

    pub fn get<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> std::io::Result<Vec<E2>> {
        let referers = Relation::referers::<E1, E2>(e1, db)?;
        Ok(E2::get_each_u8(&referers, db))
    }

    pub fn get_one<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> std::io::Result<E2> {
        let referers = Relation::referers::<E1, E2>(e1, db)?;
        if referers.len() > 0 {
            match E2::get_from_u8_array(&referers[0], db)? {
                Some(e) => Ok(e),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Relations for this entity are empty",
                )),
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Relations for this entity are empty",
            ))
        }
    }

    fn create_link<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> std::io::Result<()> {
        let tree = db.open_tree(Relation::tree_name::<E1, E2>())?;
        if let Some(vec) = tree.get(e1.get_key().as_bytes())? {
            let mut key_list: Vec<Vec<u8>> = bincode::deserialize(vec.as_ref()).unwrap();
            key_list.push(e2.get_key().as_bytes());
            tree.insert(
                e1.get_key().as_bytes(),
                bincode::serialize(&key_list).unwrap(),
            )?;
        }
        Ok(())
    }

    fn remove_link_with_keys<E1: Entity, E2: Entity>(
        e1: &[u8],
        e2: &[u8],
        db: &Db,
    ) -> std::io::Result<()> {
        let tree = db.open_tree(Relation::tree_name::<E1, E2>())?;
        if let Some(vec) = tree.get(e1)? {
            let mut key_list: Vec<Vec<u8>> = bincode::deserialize(vec.as_ref()).unwrap();
            if let Some(index) = key_list
                .iter()
                .position(|value| value.to_ascii_lowercase() == e2.to_ascii_lowercase())
            {
                key_list.swap_remove(index);
            }
            tree.insert(e1, bincode::serialize(&key_list).unwrap())?;
        }
        Ok(())
    }
    
    fn remove_link<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> std::io::Result<()> {
        Relation::remove_link_with_keys::<E1, E2>(
            &e1.get_key().as_bytes(),
            &e2.get_key().as_bytes(),
            db,
        )
    }
}
