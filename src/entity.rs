use std::{fs::File, io::ErrorKind, mem::size_of};

use crate::relation::{DeletionBehaviour, FamilyDescriptor, Relation, RelationDescriptor};
use serde::{de::DeserializeOwned, Serialize};
use sled::{Batch, Db, IVec, Tree};
use std::convert::TryInto;

/// The `Entity` trait provides document store capabilities for any struct that implements it.
pub trait Entity: Serialize + DeserializeOwned {
    /// The type of the Key for this document store.
    ///
    /// It needs to implement the [`AsBytes`](entity/trait.AsBytes.html) and the Clone traits
    /// which are already implemented for
    ///  - `String`
    ///  - `u32`
    ///  - `u64`
    ///  - `i32`
    ///  - `i64`
    type Key: AsBytes + Clone;

    /// The name of the store, as a string.
    /// It represents a keyspace in the database. It needs to be unique for the struct that implements it.
    ///
    /// A recommendation is to return the name of the struct in `snake_case`.
    /// ### Example
    /// ```rs
    /// impl Entity for MyStruct {
    ///     fn store_name() -> &'static str {
    ///         "my_struct"
    ///     }
    /// }
    /// ```
    fn store_name() -> &'static str;

    /// A function that returns a reference to the key for this entity instance.
    ///
    /// ### Example
    /// ```rs
    /// impl Entity for MyStruct {
    ///     fn get_key(&self) -> &Self::Key {
    ///         &self.key
    ///     }
    /// }
    /// ```
    fn get_key(&self) -> &Self::Key;

    /// A function that changes this entity instance's key to another key.
    /// This is used on behalf of the user when using 
    /// [`save_child`](entity/trait.Entity.html#method.save_child), 
    /// [`save_sibling`](entity/trait.Entity.html#method.save_sibling) and 
    /// [`save_next`](entity/trait.AutoIncrementEntity.html#tymethod.save_next)
    ///
    /// ### Example
    /// ```rs
    /// impl Entity for MyStruct {
    ///     fn set_key(&mut self, key : &Self::Key) {
    ///         self.key = key.clone();
    ///     }
    /// }
    /// ```
    fn set_key(&mut self, key: &Self::Key);

    /// A function that returns the list of sibling trees as well as the 
    /// [`DeletionBehaviour`](relation/enum.DeletionBehaviour.html) to use 
    /// for the sibling counterparts of this instance if it is removed
    /// 
    /// Override it to create one or several sibling relationships.
    /// 
    /// ⚠ Note that you should also override it in the sibling entity implementations
    /// with this one's `store_name` along with the `DeletionBehaviour`. It should
    /// **not** bet set to `DeletionBehaviour::Error` to avoid creating a deadlock.
    ///
    /// ### Example
    /// ```rs
    /// impl Entity for MyStruct {
    ///     fn get_sibling_trees() -> Vec<(&'static str, DeletionBehaviour)> {
    ///         vec![
    ///             ("sibling_struct_1",DeletionBehaviour::Cascade),
    ///             ("sibling_struct_2",DeletionBehaviour::Error)
    ///         ]
    ///     }
    /// }
    /// ```
    fn get_sibling_trees() -> Vec<(&'static str, DeletionBehaviour)> {
        Vec::new()
    }

    /// A function that returns the list of child trees as well as the 
    /// [`DeletionBehaviour`](relation/enum.DeletionBehaviour.html) to use 
    /// for the child instances of this instance if it is removed
    /// 
    /// Override it to create one or several child relationships.
    /// 
    /// ⚠ Note that you should not use `DeletionBehaviour::BreakLink` here
    /// for integrity's sake, but it remains possible.
    /// 
    /// Contrary to sibling relationships, nothing needs to be done in the child
    /// Entity implementation
    ///
    /// ### Example
    /// ```rs
    /// impl Entity for MyStruct {
    ///     fn get_child_trees() -> Vec<(&'static str, DeletionBehaviour)> {
    ///         vec![
    ///             ("child_struct",DeletionBehaviour::Cascade),
    ///         ]
    ///     }
    /// }
    /// ```
    fn get_child_trees() -> Vec<(&'static str, DeletionBehaviour)> {
        Vec::new()
    }

    /// Call this function once the database is opened on each Entity that you want to use.
    /// This is necessary to provide safe and type-agnostic deletion mechanisms.
    /// 
    /// ⚠ If this function is not called, deleting an entity of that type will result in an error.
    /// 
    /// ### Example
    /// 
    /// ```rs
    /// impl Entity for MyStruct { /* ... */}
    /// ```
    /// 
    /// ```rs
    /// MyStruct::register(&db)?;
    /// ```
    fn register(db: &Db) -> std::io::Result<()> {
        let desc = FamilyDescriptor {
            tree_name: String::from(Self::store_name()),
            child_trees: Self::get_child_trees()
                .iter()
                .map(|e| (String::from(e.0), e.1))
                .collect(),
            sibling_trees: Self::get_sibling_trees()
                .iter()
                .map(|e| (String::from(e.0), e.1))
                .collect(),
        };
        desc.save(db)
    }

    #[doc(hidden)]
    fn get_tree(db: &Db) -> std::io::Result<Tree> {
        db.open_tree(Self::store_name())
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not open tree"))
    }

    #[doc(hidden)]
    fn from_ivec(vec: IVec) -> Self {
        bincode::deserialize::<Self>(vec.as_ref()).unwrap()
    }

    #[doc(hidden)]
    fn to_ivec(&self) -> IVec {
        IVec::from(bincode::serialize(self).unwrap())
    }

    /// 
    fn get(key: &Self::Key, db: &Db) -> std::io::Result<Option<Self>> {
        Self::get_from_u8_array(&key.as_bytes(), db)
    }

    fn get_count(db: &Db) -> std::io::Result<usize> {
        Ok(Self::get_tree(db)?.len())
    }

    fn get_from_u8_array(key: &[u8], db: &Db) -> std::io::Result<Option<Self>> {
        Ok(Self::get_tree(db)?
            .get(key)
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not search tree"))?
            .map(|vec| Self::from_ivec(vec)))
    }

    fn get_with_prefix(key: &impl AsBytes, db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .scan_prefix(key.as_bytes())
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    fn get_in_range(start: impl AsBytes, end: impl AsBytes, db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .range(start.as_bytes()..end.as_bytes())
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    fn get_from_start(
        start: usize,
        offset: usize,
        prefix: Option<impl AsBytes>,
        db: &Db,
    ) -> std::io::Result<Vec<Self>> {
        let mut iter = if let Some(prefix) = prefix {
            Self::get_tree(db)?.scan_prefix(prefix.as_bytes())
        } else {
            Self::get_tree(db)?.iter()
        };
        let mut result = Vec::new();
        for i in 0..(start + offset) {
            match iter.next() {
                Some(e) => {
                    if i >= start {
                        result.push(Self::from_ivec(e.unwrap().1));
                    }
                }
                None => return Ok(result),
            }
        }
        Ok(result)
    }

    fn get_from_end(
        start: usize,
        offset: usize,
        prefix: Option<impl AsBytes>,
        db: &Db,
    ) -> std::io::Result<Vec<Self>> {
        let mut iter = if let Some(prefix) = prefix {
            Self::get_tree(db)?.scan_prefix(prefix.as_bytes())
        } else {
            Self::get_tree(db)?.iter()
        };
        let mut result = Vec::new();
        for i in 0..(start + offset) {
            match iter.next_back() {
                Some(e) => {
                    if i >= start {
                        result.push(Self::from_ivec(e.unwrap().1));
                    }
                }
                None => break,
            }
        }
        result.reverse();
        Ok(result)
    }

    fn get_with_filter<F: Fn(&Self) -> bool>(f: F, db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .iter()
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .filter(|e| f(e))
            .collect())
    }

    fn get_all(db: &Db) -> std::io::Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .iter()
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    fn get_each(keys: &[Self::Key], db: &Db) -> Vec<Self> {
        keys.iter()
            .map(|key| Self::get(key, db))
            .filter_map(|res| match res {
                Ok(opt) => opt,
                Err(_) => None,
            })
            .collect()
    }

    fn get_each_u8(keys: &[Vec<u8>], db: &Db) -> Vec<Self> {
        keys.iter()
            .map(|key| Self::get_from_u8_array(key, db))
            .filter_map(|res| match res {
                Ok(opt) => opt,
                Err(_) => None,
            })
            .collect()
    }

    fn save(&self, db: &Db) -> std::io::Result<()> {
        Self::get_tree(db)?.insert(
            &self.get_key().as_bytes(),
            bincode::serialize(self).unwrap(),
        )?;
        Ok(())
    }

    fn update<F: Fn(&mut Self)>(key: &Self::Key, f: F, db: &Db) -> std::io::Result<()> {
        Self::get_tree(db)?
            .fetch_and_update(&key.as_bytes(), |e| {
                e.map(|u8_arr| {
                    let mut value: Self = Self::from_ivec(IVec::from(u8_arr));
                    f(&mut value);
                    value.to_ivec()
                })
            })
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "Could not update object")
            })?;
        Ok(())
    }

    #[doc(hidden)]
    fn pre_remove(key: &[u8], db: &Db) -> std::io::Result<()> {
        let mut to_be_removed = RelationDescriptor::default();
        Relation::can_be_deleted(Self::store_name(), key, &Vec::new(), &mut to_be_removed, db)?;
        for (tree, keys) in &to_be_removed.related_entities {
            let tree = db.open_tree(tree)?;
            let mut batch = Batch::default();
            keys.iter().for_each(|(k, _)| batch.remove(k.as_slice()));
            tree.apply_batch(batch)?;
        }
        Relation::remove_entity_entry::<Self>(key, db)?;
        Ok(())
    }

    fn can_be_removed(key: &[u8], db: &Db) -> std::io::Result<()> {
        Relation::can_be_deleted(
            Self::store_name(),
            key,
            &Vec::new(),
            &mut RelationDescriptor::default(),
            db,
        )?;
        Ok(())
    }

    fn remove(key: &Self::Key, db: &Db) -> std::io::Result<()> {
        Self::remove_from_u8_array(&key.as_bytes(), db)
    }

    fn remove_from_u8_array(key: &[u8], db: &Db) -> std::io::Result<()> {
        Self::pre_remove(key, db)?;
        Self::get_tree(db)?.remove(key)?;
        Ok(())
    }

    #[doc(hidden)]
    fn remove_prefixed(prefix: impl AsBytes, db: &Db) -> std::io::Result<()> {
        Self::remove_prefixed_in_tree(Self::store_name(), &prefix.as_bytes(), db)
    }

    #[doc(hidden)]
    fn remove_prefixed_in_tree(tree_name: &str, prefix: &[u8], db: &Db) -> std::io::Result<()> {
        let tree = db.open_tree(tree_name)?;
        let mut batch = Batch::default();
        tree.scan_prefix(prefix).for_each(|elem| {
            if let Ok((key, _)) = elem {
                if Self::pre_remove(&key, db).is_ok() {
                    batch.remove(key)
                }
            }
        });
        tree.apply_batch(batch)?;
        Ok(())
    }

    fn filter_remove<F: Fn(&Self) -> bool>(f: F, db: &Db) -> std::io::Result<Vec<Self>> {
        let res = Self::get_with_filter(f, db)?;
        for entity in &res {
            if Self::pre_remove(&entity.get_key().as_bytes(), db).is_ok() {
                Self::remove(&entity.get_key(), db)?;
            }
        }
        Ok(res)
    }

    fn filter_update<F: Fn(&Self) -> bool, M: Fn(&mut Self)>(
        filter: F,
        modifier: M,
        db: &Db,
    ) -> std::io::Result<()> {
        let mut res = Self::get_with_filter(filter, db)?;
        for entity in &mut res {
            modifier(entity);
            entity.save(db)?;
        }
        Ok(())
    }

    fn exists(key: &Self::Key, db: &Db) -> std::io::Result<bool> {
        Self::get_tree(db)?
            .contains_key(&key.as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    fn export_json(f: File, db: &Db) -> std::io::Result<()> {
        let all = Self::get_all(db)?;
        serde_json::to_writer(f, &all)
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not serialize objects"))
    }

    fn import_json(f: File, db: &Db) -> std::io::Result<()> {
        let all: Vec<Self> = serde_json::from_reader(f)
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "Could not deserialize objects"))?;
        for each in all {
            each.save(db)?;
        }
        Ok(())
    }

    fn create_relation<E: Entity>(
        &self,
        other: &E,
        self_to_other: DeletionBehaviour,
        other_to_self: DeletionBehaviour,
        db: &Db,
    ) -> std::io::Result<()> {
        Relation::create(self, other, self_to_other, other_to_self, db)
    }

    fn remove_relation<E: Entity>(&self, other: &E, db: &Db) -> std::io::Result<()> {
        Relation::remove(self, other, db)
    }

    fn remove_relation_with_key<E: Entity>(&self, other: &[u8], db: &Db) -> std::io::Result<()> {
        Relation::remove_by_keys::<Self, E>(&self.get_key().as_bytes(), other, db)
    }

    fn get_related<E: Entity>(&self, db: &Db) -> std::io::Result<Vec<E>> {
        Relation::get::<Self, E>(self, db)
    }

    fn get_single_related<E: Entity>(&self, db: &Db) -> std::io::Result<E> {
        Relation::get_one::<Self, E>(self, db)
    }

    fn save_sibling<E: Entity<Key = Self::Key>>(
        &self,
        sibling: &mut E,
        db: &Db,
    ) -> std::io::Result<()> {
        sibling.set_key(self.get_key());
        sibling.save(db)
    }

    fn get_sibling<E: Entity<Key = Self::Key>>(&self, db: &Db) -> std::io::Result<Option<E>> {
        E::get(&self.get_key(), db)
    }

    fn save_child<E: Entity<Key = (Self::Key, u32)>>(
        &self,
        child: &mut E,
        db: &Db,
    ) -> std::io::Result<E::Key> {
        let increment = match E::get_tree(db)?.last()? {
            Some((key, _)) => {
                let u32_part = key
                    .iter()
                    .rev()
                    .take(size_of::<u32>())
                    .rev()
                    .copied()
                    .collect::<Vec<u8>>();
                u32::from_be_bytes(u32_part.try_into().unwrap()) + 1
            }
            None => Default::default(),
        };
        let key = (self.get_key().clone(), increment);
        child.set_key(&key);
        child.save(db)?;
        Ok(key)
    }

    fn get_children<E: Entity<Key = (Self::Key, u32)>>(&self, db: &Db) -> std::io::Result<Vec<E>> {
        E::get_with_prefix(self.get_key(), db)
    }
}

pub trait AutoIncrementEntity: Entity<Key = u32> {
    fn get_next_key(db: &Db) -> std::io::Result<u32>;
    fn save_next(&mut self, db: &Db) -> std::io::Result<u32>;
}

impl<T> AutoIncrementEntity for T
where
    T: Entity<Key = u32>,
{
    fn get_next_key(db: &Db) -> std::io::Result<u32> {
        match Self::get_tree(db)?.last()? {
            Some((key, _)) => Ok(u32::from_be_bytes(key.as_ref().try_into().unwrap()) + 1),
            None => Ok(Default::default()),
        }
    }

    fn save_next(&mut self, db: &Db) -> std::io::Result<u32> {
        let next_key = Self::get_next_key(db)?;
        self.set_key(&next_key);
        self.save(db)?;
        Ok(next_key)
    }
}

pub trait AsBytes {
    fn as_bytes(&self) -> Vec<u8>;
}

impl AsBytes for String {
    fn as_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_owned()
    }
}

impl AsBytes for u32 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl AsBytes for u64 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl AsBytes for i32 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl AsBytes for i64 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl AsBytes for Vec<u8> {
    fn as_bytes(&self) -> Vec<u8> {
        self.clone()
    }
}

impl AsBytes for &[u8] {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl<K1, K2> AsBytes for (K1, K2)
where
    K1: AsBytes,
    K2: AsBytes,
{
    fn as_bytes(&self) -> Vec<u8> {
        vec![self.0.as_bytes(), self.1.as_bytes()].concat()
    }
}
