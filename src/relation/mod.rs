mod descriptor;

use sled::Db;

use crate::entity::{AsBytes, Entity};

use self::descriptor::RelationDescriptor;

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
        Relation::remove_link_with_keys::<E2, E1>(e2, e1, db)?;
        Ok(())
    }

    pub fn relations<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> std::io::Result<RelationDescriptor> {
        Relation::get_descriptor(e1, db)
    }

    pub fn has_referers<E1: Entity>(e1: &E1, db: &Db) -> std::io::Result<bool> {
        let descriptor = Self::get_descriptor(e1, db)?;
        if descriptor.related_entities.len() > 0 {
            return Ok(true);
        }
        for tree in descriptor.children_trees {
            let tree = db.open_tree(&tree)?;
            if tree.scan_prefix(&e1.get_key().as_bytes()).count() > 0 {
                return Ok(true);
            }
        }
        for tree in descriptor.sibling_trees {
            let tree = db.open_tree(&tree)?;
            if tree.contains_key(&e1.get_key().as_bytes())? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn get<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> std::io::Result<Vec<E2>> {
        let referers = Relation::relations::<E1, E2>(e1, db)?;
        if let Some(related_keys) = referers.related_entities.get(E2::tree_name()) {
            Ok(E2::get_each_u8(&related_keys, db))
        }
        else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound,"No related entities were found."))
        }
        
    }

    pub fn get_one<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> std::io::Result<E2> {
        let referers = Relation::relations::<E1, E2>(e1, db)?;
        if let Some(related_keys) = referers.related_entities.get(E2::tree_name()) {
            if related_keys.len() > 0 {
                if let Some(e) = E2::get_from_u8_array(&related_keys[0], db)? {
                    return Ok(e)
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No related entities were found.",
        ))
    }

    fn tree_name<E: Entity>() -> String {
        format!("__rel_{}", E::tree_name())
    }

    fn get_descriptor_with_key<E : Entity>(e : &[u8], db : &Db) -> std::io::Result<RelationDescriptor> {
        let tree = db.open_tree(Relation::tree_name::<E>())?;
        match tree.get(e)? {
            Some(relation_descriptor) => {
                Ok(bincode::deserialize::<RelationDescriptor>(&relation_descriptor).unwrap())
            },
            None => Ok(RelationDescriptor::new()),
        }
    }

    fn get_descriptor<E : Entity>(e : &E, db : &Db) -> std::io::Result<RelationDescriptor> {
        Self::get_descriptor_with_key::<E>(&e.get_key().as_bytes(), db)
    }

    fn save_descriptor_with_key<E : Entity>(e : &[u8], r_d : &RelationDescriptor, db : &Db) -> std::io::Result<()> {
        let tree = db.open_tree(Relation::tree_name::<E>())?;
        tree.insert(e, bincode::serialize(r_d).unwrap())?;
        Ok(())
    }

    pub fn save_descriptor<E : Entity>(e : &E, r_d : &RelationDescriptor, db : &Db) -> std::io::Result<()> {
        Self::save_descriptor_with_key::<E>(&e.get_key().as_bytes(), r_d, db)
    }

    fn create_link<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> std::io::Result<()> {
        let mut e1_descriptor = Self::get_descriptor(e1,db)?;
        e1_descriptor.add_related(e2);
        Self::save_descriptor(e1, &e1_descriptor, db)?;
        Ok(())
    }

    fn remove_link_with_keys<E1: Entity, E2: Entity>(
        e1: &[u8],
        e2: &[u8],
        db: &Db,
    ) -> std::io::Result<()> {
        let mut e1_descriptor = Self::get_descriptor_with_key::<E1>(e1, db)?;
        e1_descriptor.remove_related_by_key::<E2>(e2);
        Self::save_descriptor_with_key::<E1>(e1, &e1_descriptor, db)?;
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
