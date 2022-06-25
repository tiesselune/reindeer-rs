mod descriptor;

use crate::error::Result;
use crate::{Error, ErrorKind};
use crate::entity::{AsBytes, Entity};
use serde_derive::{Deserialize, Serialize};
use sled::Db;

pub use self::descriptor::FamilyDescriptor;
pub use self::descriptor::{EntityRelations, RelationMap};

pub struct Relation;

impl Relation {
    pub fn create<E1: Entity, E2: Entity>(
        e1: &E1,
        e2: &E2,
        e1_to_e2: DeletionBehaviour,
        e2_to_e1: DeletionBehaviour,
        name : Option<&str>,
        db: &Db,
    ) -> Result<()> {
        Relation::create_link(e1, e2, e1_to_e2,name, db)?;
        Relation::create_link(e2, e1, e2_to_e1,name, db)?;
        Ok(())
    }

    pub fn remove<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> Result<()> {
        Relation::remove_link(e1, e2, db)?;
        Relation::remove_link(e2, e1, db)?;
        Ok(())
    }

    pub fn remove_entity_entry<E1: Entity>(key: &[u8], db: &Db) -> Result<()> {
        let descriptor = Self::get_descriptor_with_key::<E1>(key, db)?;
        for (tree_name, referers) in descriptor.related_entities {
            for referer in referers {
                Self::remove_link_with_keys_and_tree_names(
                    &tree_name,
                    &referer.key,
                    E1::store_name(),
                    key,
                    db,
                )?;
            }
        }
        let tree = db.open_tree(Relation::tree_name(E1::store_name()))?;
        tree.remove(key)?;
        Ok(())
    }

    pub fn remove_by_keys<E1: Entity, E2: Entity>(
        e1: &[u8],
        e2: &[u8],
        db: &Db,
    ) -> Result<()> {
        Relation::remove_link_with_keys::<E1, E2>(e1, e2, db)?;
        Relation::remove_link_with_keys::<E2, E1>(e2, e1, db)?;
        Ok(())
    }

    pub fn remove_by_keys_and_tree_names(
        tree1: &str,
        e1: &[u8],
        tree2: &str,
        e2: &[u8],
        db: &Db,
    ) -> Result<()> {
        Relation::remove_link_with_keys_and_tree_names(tree1, e1, tree2, e2, db)?;
        Relation::remove_link_with_keys_and_tree_names(tree2, e2, tree1, e1, db)?;
        Ok(())
    }

    pub fn relations<E1: Entity>(e1: &E1, db: &Db) -> Result<EntityRelations> {
        Relation::get_descriptor(e1, db)
    }

    pub fn relations_with_key<E1: Entity>(
        key: &[u8],
        db: &Db,
    ) -> Result<EntityRelations> {
        Relation::get_descriptor_with_key::<E1>(key, db)
    }

    pub fn can_be_deleted(
        tree_name: &str,
        e1: &[u8],
        already_checked: &[(String, Vec<u8>)],
        removable_entities: &mut EntityRelations,
        db: &Db,
    ) -> Result<()> {
        if already_checked
            .iter()
            .any(|(tn, k)| tn == tree_name && k == e1)
        {
            return Ok(());
        }
        let descriptor = Self::get_descriptor_with_key_and_tree_name(tree_name, e1, db)?;
        let family_descriptor = FamilyDescriptor::get(&String::from(tree_name), db)?;

        for (other_tree_name, entities) in &descriptor.related_entities {
            for rd in entities {
                match rd.deletion_behaviour {
                    DeletionBehaviour::Error => {
                        if already_checked
                            .iter()
                            .any(|(tn, k)| tn == other_tree_name && k.as_bytes() == rd.key.as_bytes())
                        {
                            continue;
                        }
                        return Err(Error::new(
                            ErrorKind::IntegrityError,
                            format!("Constrained related entity exists in {}", other_tree_name),
                        ));
                    }
                    DeletionBehaviour::Cascade => {
                        let mut new_already_checked = already_checked.to_owned();
                        new_already_checked.push((String::from(tree_name), e1.to_vec()));
                        Self::can_be_deleted(
                            other_tree_name,
                            &rd.key,
                            &new_already_checked,
                            removable_entities,
                            db,
                        )?;
                        removable_entities.add_related_by_key(
                            other_tree_name,
                            &rd.key,
                            DeletionBehaviour::Cascade,
                            None,
                        );
                    }
                    _ => {}
                }
            }
        }
        if family_descriptor.is_none() {
            return Err(Error::new(
                ErrorKind::UnregisteredEntity,
                format!("Trying to use unregistered entity {}", tree_name),
            ));
        }
        let family_descriptor = family_descriptor.unwrap();
        for (other_tree_name, behaviour) in &family_descriptor.sibling_trees {
            match behaviour {
                DeletionBehaviour::Error => {
                    if already_checked
                        .iter()
                        .any(|(tn, k)| tn == other_tree_name && k == e1)
                    {
                        continue;
                    }
                    let tree = db.open_tree(&other_tree_name)?;
                    if tree.contains_key(e1)? {
                        return Err(Error::new(
                            ErrorKind::IntegrityError,
                            format!("Constrained sibling entity exists in {}", &other_tree_name),
                        ));
                    }
                }
                DeletionBehaviour::Cascade => {
                    let mut new_already_checked = already_checked.to_owned();
                    new_already_checked.push((String::from(tree_name), e1.to_vec()));
                    Self::can_be_deleted(
                        other_tree_name,
                        e1,
                        &new_already_checked,
                        removable_entities,
                        db,
                    )?;
                    removable_entities.add_related_by_key(
                        other_tree_name,
                        e1,
                        DeletionBehaviour::Cascade,
                        None,
                    );
                }
                _ => {}
            }
        }
        for (other_tree_name, behaviour) in &family_descriptor.child_trees {
            match behaviour {
                DeletionBehaviour::Error => {
                    let tree = db.open_tree(&other_tree_name)?;
                    if tree.scan_prefix(e1).count() > 0 {
                        return Err(Error::new(
                            ErrorKind::IntegrityError,
                            format!("Constrained child entity exists in {}", &other_tree_name),
                        ));
                    }
                }
                DeletionBehaviour::Cascade => {
                    let mut new_already_checked = already_checked.to_owned();
                    new_already_checked.push((String::from(tree_name), e1.to_vec()));
                    let tree = db.open_tree(&other_tree_name)?;
                    let keys = tree
                        .scan_prefix(e1)
                        .filter_map(|e| {
                            if let Ok((key, _)) = e {
                                Some(key.to_vec())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Vec<u8>>>();
                    for key in keys {
                        Self::can_be_deleted(
                            other_tree_name,
                            &key,
                            &new_already_checked.clone(),
                            removable_entities,
                            db,
                        )?;
                        removable_entities.add_related_by_key(
                            other_tree_name,
                            &key,
                            DeletionBehaviour::Cascade,
                            None,
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn get<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> Result<Vec<E2>> {
        let referers = Relation::relations(e1, db)?;
        if let Some(related_keys) = referers.related_entities.get(E2::store_name()) {
            Ok(E2::get_each_u8(
                (related_keys
                    .iter()
                    .map(|e| e.key.clone())
                    .collect::<Vec<Vec<u8>>>())
                .as_slice(),
                db,
            ))
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_with_name<E1 : Entity, E2 : Entity>(e1 : &E1, name : &str, db : &Db) -> Result<Vec<E2>> {
        let referers = Relation::relations(e1, db)?;
        if let Some(related_keys) = referers.related_entities.get(E2::store_name()) {
            Ok(E2::get_each_u8(
                (related_keys
                    .iter()
                    .filter(|rd| match &rd.name {Some(n) => name == n, None => false})
                    .map(|e| e.key.clone())
                    .collect::<Vec<Vec<u8>>>())
                .as_slice(),
                db,
            ))
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_one<E1: Entity, E2: Entity>(e1: &E1, db: &Db) -> Result<Option<E2>> {
        let referers = Relation::relations(e1, db)?;
        if let Some(related_keys) = referers.related_entities.get(E2::store_name()) {
            if !related_keys.is_empty() {
                E2::get_from_u8_array(&related_keys[0].key, db)
            }
            else {
                Ok(None)
            }
        }
        else{
            Ok(None)
        }
    }

    pub fn get_one_with_name<E1: Entity, E2: Entity>(e1 : &E1, name : &str, db : &Db) -> Result<Option<E2>>{
        let referers = Relation::relations(e1, db)?;
        if let Some(related_keys) = referers.related_entities.get(E2::store_name()) {
            let item = related_keys.iter().find(|rd| if let Some(n) = &rd.name { name == n } else {false});
            match item {
                Some(rd) => {
                    E2::get_from_u8_array(&related_keys[0].key, db)
                },
                None => Ok(None),
                
            }
        }
        else {
            Ok(None)
        }
    }

    fn tree_name(entity_tree: &str) -> String {
        format!("__$rel_{}", entity_tree)
    }

    fn get_descriptor_with_key_and_tree_name(
        tree_name: &str,
        e: &[u8],
        db: &Db,
    ) -> Result<EntityRelations> {
        let tree = db.open_tree(Relation::tree_name(tree_name))?;
        match tree.get(e)? {
            Some(relation_descriptor) => {
                Ok(bincode::deserialize::<EntityRelations>(&relation_descriptor).unwrap())
            }
            None => Ok(EntityRelations::default()),
        }
    }

    fn get_descriptor_with_key<E: Entity>(
        e: &[u8],
        db: &Db,
    ) -> Result<EntityRelations> {
        Self::get_descriptor_with_key_and_tree_name(E::store_name(), e, db)
    }

    fn get_descriptor<E: Entity>(e: &E, db: &Db) -> Result<EntityRelations> {
        Self::get_descriptor_with_key::<E>(&e.get_key().as_bytes(), db)
    }

    fn save_descriptor_with_key<E: Entity>(
        e: &[u8],
        r_d: &EntityRelations,
        db: &Db,
    ) -> Result<()> {
        let tree = db.open_tree(Relation::tree_name(E::store_name()))?;
        tree.insert(e, bincode::serialize(r_d).unwrap())?;
        Ok(())
    }

    fn save_descriptor_with_key_and_tree_name(
        tree_name: &str,
        e: &[u8],
        r_d: &EntityRelations,
        db: &Db,
    ) -> std::io::Result<()> {
        let tree = db.open_tree(Relation::tree_name(tree_name))?;
        tree.insert(e, bincode::serialize(r_d).unwrap())?;
        Ok(())
    }

    pub fn save_descriptor<E: Entity>(
        e: &E,
        r_d: &EntityRelations,
        db: &Db,
    ) -> Result<()> {
        Self::save_descriptor_with_key::<E>(&e.get_key().as_bytes(), r_d, db)
    }

    fn create_link<E1: Entity, E2: Entity>(
        e1: &E1,
        e2: &E2,
        e1_to_e2: DeletionBehaviour,
        name : Option<&str>,
        db: &Db,
    ) -> Result<()> {
        let mut e1_descriptor = Self::get_descriptor(e1, db)?;
        e1_descriptor.add_related(e2, e1_to_e2,name);
        Self::save_descriptor(e1, &e1_descriptor, db)?;
        Ok(())
    }

    fn remove_link_with_keys<E1: Entity, E2: Entity>(
        e1: &[u8],
        e2: &[u8],
        db: &Db,
    ) -> Result<()> {
        let mut e1_descriptor = Self::get_descriptor_with_key::<E1>(e1, db)?;
        e1_descriptor.remove_related_by_key::<E2>(e2);
        Self::save_descriptor_with_key::<E1>(e1, &e1_descriptor, db)?;
        Ok(())
    }

    fn remove_link_with_keys_and_tree_names(
        tree1: &str,
        e1: &[u8],
        tree2: &str,
        e2: &[u8],
        db: &Db,
    ) -> Result<()> {
        let mut e1_descriptor = Self::get_descriptor_with_key_and_tree_name(tree1, e1, db)?;
        e1_descriptor.remove_related_by_key_and_tree_name(tree2, e2);
        Self::save_descriptor_with_key_and_tree_name(tree1, e1, &e1_descriptor, db)?;
        Ok(())
    }

    fn remove_link<E1: Entity, E2: Entity>(e1: &E1, e2: &E2, db: &Db) -> Result<()> {
        Relation::remove_link_with_keys::<E1, E2>(
            &e1.get_key().as_bytes(),
            &e2.get_key().as_bytes(),
            db,
        )
    }
}

/// Enum for use in relation description, defining how the database must behave if one end of the relation is removed.
#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Debug)]
pub enum DeletionBehaviour {
    /// Trying to remove the current entity while a related one still exists will result in an error
    Error,
    /// Related entities are left untouched, but the link between the two entities is removed
    BreakLink,
    /// Related entities are also removed if the current one is removed
    Cascade,
}
