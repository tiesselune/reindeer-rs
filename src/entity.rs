//! # Entity Module
//! This module provides the `Entity` trait as well as other utilities to manipulate entities and entity stores.
//! For relation-related definitions, take a look a the [`relation` module](relation/index.html).

use std::{fs::File, mem::size_of};

use crate::relation::{DeletionBehaviour, FamilyDescriptor, Relation, EntityRelations};
use serde::{de::DeserializeOwned, Serialize};
use sled::{Batch, Db, IVec, Tree};
use std::convert::TryInto;
use crate::error::Result;

/// The `Entity` trait provides document store capabilities for any struct that implements it.
/// 
/// ### Example
/// ```rust
/// use reindeer::{Entity, Serialize,Deserialize,open};
/// 
/// #[derive(Serialize,Deserialize)]
/// struct MyStruct  { pub key : u32, pub prop1 : String }
/// 
/// impl Entity for MyStruct{
///    type Key = u32;
///    fn store_name() -> &'static str {
///        "my-struct"
///    }
///    fn get_key(&self) -> &Self::Key {
///        &self.key
///    }
///    fn set_key(&mut self, key : &Self::Key) {
///        self.key = key.clone();
///    }
/// }
/// ```
/// 
/// ```rust
/// let db = open("./my-db")?;
/// let my_struct = MyStruct { key : 2 , prop1 : String::from("Hello, World!")};
/// my_struct.save(&db)?;
/// ```
/// ```rust
/// let my_struct_0 = MyStruct::get(&2,&db)?;
/// ```
/// 
/// More information on how to use the trait is provided below.
/// 
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
    /// ```rust
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
    /// ```rust
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
    /// ```rust
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
    /// ```rust
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
    /// ```rust
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
    /// ```rust
    /// impl Entity for MyStruct { /* ... */}
    /// ```
    /// 
    /// ```rust
    /// MyStruct::register(&db)?;
    /// ```
    fn register(db: &Db) -> Result<()> {
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
        desc.save(db)?;
        Ok(())
    }

    #[doc(hidden)]
    fn get_tree(db: &Db) -> Result<Tree> {
        Ok(db.open_tree(Self::store_name())?)
    }

    #[doc(hidden)]
    fn from_ivec(vec: IVec) -> Self {
        bincode::deserialize::<Self>(vec.as_ref()).unwrap()
    }

    #[doc(hidden)]
    fn to_ivec(&self) -> IVec {
        IVec::from(bincode::serialize(self).unwrap())
    }

    /// Retrieves an entity instance given its key.
    /// 
    /// If the key does not exist, it returns None.
    /// 
    /// ### Example
    /// 
    /// ```rust
    /// if let Some(my_struct_4) = MyStruct::get(&4,&db)? {
    ///     /* ... */
    /// }
    /// ```
    fn get(key: &Self::Key, db: &Db) -> Result<Option<Self>> {
        Ok(Self::get_from_u8_array(&key.as_bytes(), db)?)
    }

    /// Retrieves all entities of a given type.
    /// 
    /// If a lot of entities are registered to the database, this
    /// might be very heavy on resources.
    /// 
    /// 
    /// ### Example
    /// 
    /// ```rust
    /// let entities = MyStruct::get_all(&db)?;
    /// ```
    fn get_all(db: &Db) -> Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .iter()
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    /// Returns the number of saved instances for this entity type.
    /// 
    /// ### Example
    /// ```rust
    /// let count = MyStruct::get_count()?;
    /// ```
    fn get_count(db: &Db) -> Result<usize> {
        Ok(Self::get_tree(db)?.len())
    }

    #[doc(hidden)]
    fn get_from_u8_array(key: &[u8], db: &Db) -> Result<Option<Self>> {
        Ok(Self::get_tree(db)?
            .get(key)?
            .map(|vec| Self::from_ivec(vec)))
    }

    #[doc(hidden)]
    fn get_with_prefix(key: &impl AsBytes, db: &Db) -> Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .scan_prefix(key.as_bytes())
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    /// Gets entities in a range of keys with a min and max values
    /// This can be especially useful when keys are integral types,
    /// but any key will work.
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_in_range(10,30,&db)?;
    /// ```
    fn get_in_range(start: impl AsBytes, end: impl AsBytes, db: &Db) -> Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .range(start.as_bytes()..end.as_bytes())
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .collect())
    }

    /// Gets `count` entities starting at the instance at index `start` in the given store
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_from_start(10,20,None,&db)?;
    /// ```
    /// ## Child entities
    /// 
    /// A parent key can be supplied for child entities, to consider only children of a given parent.
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_from_start(10,20,Some(parent.get_key().to_owned()),&db)?;
    /// ```
    fn get_from_start(
        start: usize,
        count: usize,
        parent: Option<impl AsBytes>,
        db: &Db,
    ) -> Result<Vec<Self>> {
        let mut iter = if let Some(prefix) = parent {
            Self::get_tree(db)?.scan_prefix(prefix.as_bytes())
        } else {
            Self::get_tree(db)?.iter()
        };
        let mut result = Vec::new();
        for i in 0..(start + count) {
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

    /// Gets `count` entities starting at the instance at index `start` from the end of a given store
    /// Same as `get_from_start`, but starting at the end of the store.
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_from_end(10,20,None,&db)?;
    /// ```
    /// ## Child entities
    /// 
    /// A parent key can be supplied for child entities, to consider only children of a given parent.
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_from_end(10,20,Some(parent.get_key().to_owned()),&db)?;
    /// ```
    fn get_from_end(
        start: usize,
        offset: usize,
        prefix: Option<impl AsBytes>,
        db: &Db,
    ) -> Result<Vec<Self>> {
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

    /// Gets all entities of a given store matching a condition materialized 
    /// as a function returning a boolean
    /// 
    /// ⚠ This will effectively iterate over every entity in the store.
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_with_filter(|m_struct| m_struct.prop > 20,&db)?;
    /// ```
    fn get_with_filter<F: Fn(&Self) -> bool>(f: F, db: &Db) -> Result<Vec<Self>> {
        Ok(Self::get_tree(db)?
            .iter()
            .map(|elem| Self::from_ivec(elem.unwrap().1))
            .filter(|e| f(e))
            .collect())
    }

    /// Gets several entites matching a collection of keys
    /// 
    /// ⚠ This will call `get` as many times as the number of keys provided.
    /// 
    /// ### Example
    /// ```rust
    /// let entities = MyStruct::get_each(vec![4,8,9],&db)?;
    /// ```
    fn get_each(keys: &[Self::Key], db: &Db) -> Vec<Self> {
        keys.iter()
            .map(|key| Self::get(key, db))
            .filter_map(|res| match res {
                Ok(opt) => opt,
                Err(_) => None,
            })
            .collect()
    }

    #[doc(hidden)]
    fn get_each_u8(keys: &[Vec<u8>], db: &Db) -> Vec<Self> {
        keys.iter()
            .map(|key| Self::get_from_u8_array(key, db))
            .filter_map(|res| match res {
                Ok(opt) => opt,
                Err(_) => None,
            })
            .collect()
    }

    /// Saves an entity to the database, using its key provided by the`get_key` method.
    /// 
    /// ### Example
    /// 
    /// ```rust
    /// let my_struct = MyStruct { key : 0, prop1 : String::from("Hello"), prop2 : 554};
    /// my_struct.save(&db)?;
    /// ```
    fn save(&self, db: &Db) -> Result<()> {
        Self::get_tree(db)?.insert(
            &self.get_key().as_bytes(),
            bincode::serialize(self).unwrap(),
        )?;
        Ok(())
    }

    /// Updates an entity entry using the provided function
    /// 
    /// ### Example
    /// This will get the `MyStruct` instance with key 3  and increment its `prop1` member 
    /// ```rust
    /// MyStruct::update(&3,|my_struct| my_struct.prop1++,&db)?;
    /// ```
    fn update<F: Fn(&mut Self)>(key: &Self::Key, f: F, db: &Db) -> Result<()> {
        Self::get_tree(db)?
            .fetch_and_update(&key.as_bytes(), |e| {
                e.map(|u8_arr| {
                    let mut value: Self = Self::from_ivec(IVec::from(u8_arr));
                    f(&mut value);
                    value.to_ivec()
                })
            })?;
        Ok(())
    }

    /// Updates all entities that match a condition provided as a function
    /// 
    /// ### Example
    /// This will get all the `MyStruct` instances with prop1 greater than 100
    /// and change it to be 0 instead;
    /// ```rust
    /// MyStruct::filter_update(|my_struct| mu_struct.prop1 > 100,|my_struct| {my_struct.prop1 = 0;},&db)?;
    /// ```
    fn filter_update<F: Fn(&Self) -> bool, M: Fn(&mut Self)>(
        filter: F,
        modifier: M,
        db: &Db,
    ) -> Result<()> {
        let mut res = Self::get_with_filter(filter, db)?;
        for entity in &mut res {
            modifier(entity);
            entity.save(db)?;
        }
        Ok(())
    }

    #[doc(hidden)]
    fn pre_remove(key: &[u8], db: &Db) -> Result<()> {
        let mut to_be_removed = EntityRelations::default();
        Relation::can_be_deleted(Self::store_name(), key, &Vec::new(), &mut to_be_removed, db)?;
        for (tree, keys) in &to_be_removed.related_entities {
            let tree = db.open_tree(tree)?;
            let mut batch = Batch::default();
            keys.iter().for_each(|rd| batch.remove(rd.key.as_slice()));
            tree.apply_batch(batch)?;
        }
        Relation::remove_entity_entry::<Self>(key, db)?;
        Ok(())
    }

    #[doc(hidden)]
    fn can_be_removed(key: &[u8], db: &Db) -> Result<()> {
        Relation::can_be_deleted(
            Self::store_name(),
            key,
            &Vec::new(),
            &mut EntityRelations::default(),
            db,
        )?;
        Ok(())
    }

    /// Removes an entity given its key.
    /// ⚠ If removal is impossible due to integrity checks 
    /// (`DeletionBehaviour::Error` found in the relation hierarchy), this will result in an error.
    /// 
    /// ### Example
    /// ```rust
    /// MyStruct::remove(&3, &db);
    /// ```
    fn remove(key: &Self::Key, db: &Db) -> Result<()> {
        Self::remove_from_u8_array(&key.as_bytes(), db)
    }

    #[doc(hidden)]
    fn remove_from_u8_array(key: &[u8], db: &Db) -> Result<()> {
        Self::pre_remove(key, db)?;
        Self::get_tree(db)?.remove(key)?;
        Ok(())
    }

    #[doc(hidden)]
    fn remove_prefixed(prefix: impl AsBytes, db: &Db) -> Result<()> {
        Self::remove_prefixed_in_tree(Self::store_name(), &prefix.as_bytes(), db)
    }

    #[doc(hidden)]
    fn remove_prefixed_in_tree(tree_name: &str, prefix: &[u8], db: &Db) -> Result<()> {
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

    /// Removes every entry of a store, given a condition in the form of a function returning a boolean
    /// and returns the array of removed elements.
    /// ⚠ If removal is impossible due to integrity checks 
    /// (`DeletionBehaviour::Error` found in the relation hierarchy), they won't be deleted and won't be
    /// included in results.
    /// 
    /// ### Example
    /// ```rust
    /// MyStruct::remove(&3, &db);
    /// ```
    fn filter_remove<F: Fn(&Self) -> bool>(f: F, db: &Db) -> Result<Vec<Self>> {
        let mut res = Self::get_with_filter(f, db)?;
        let mut to_remove_from_result = Vec::new();
        for (index,entity) in res.iter().enumerate() {
            if Self::remove(&entity.get_key(), db).is_err() {
                to_remove_from_result.push(index)
            };
        }
        for index in to_remove_from_result {
            res.remove(index);
        }
        Ok(res)
    }

    /// Checks if an entity exists in a given store, without fetching it.
    /// ### Example
    /// ```rust
    /// if MyStruct::exists(&3, &db)? {
    ///     /* */
    /// }
    /// ```
    fn exists(key: &Self::Key, db: &Db) -> Result<bool> {
        Ok(Self::get_tree(db)?
            .contains_key(&key.as_bytes())?)
    }

    /// Exports the entire store for this entity as a JSON file.
    /// This can be used for saving purposes.
    fn export_json(f: File, db: &Db) -> Result<()> {
        let all = Self::get_all(db)?;
        serde_json::to_writer(f, &all)?;
        Ok(())
    }

    /// Imports the entire store for this entity as a JSON file.
    /// Any existing entities with matching keys will be overridden.
    /// 
    /// This can be used for restoring purposes.
    /// 
    /// ⚠ If the structure of the JSON file does not match the Structs used in the app, this will fail with an error.
    fn import_json(f: File, db: &Db) -> Result<()> {
        let all: Vec<Self> = serde_json::from_reader(f)?;
        for each in all {
            each.save(db)?;
        }
        Ok(())
    }

    /// Creates a free relation between this entity and another one.
    /// 
    /// As this creates a two way binding, `DeletionBehaviour` in both ways must be provided :
    ///  - `self_to_other` defines what happens to `other` if `self` gets removed from the database
    ///  - `other_to_self` defines what happens to `self` if `other` gets removed from the database
    fn create_relation<E: Entity>(
        &self,
        other: &E,
        self_to_other: DeletionBehaviour,
        other_to_self: DeletionBehaviour,
        name : Option<&str>,
        db: &Db,
    ) -> Result<()> {
        Relation::create(self, other, self_to_other, other_to_self, name,db)
    }

    /// Breaks an existing link between two entities.
    /// 
    /// This will remove the relation in both ways.
    fn remove_relation<E: Entity>(&self, other: &E, db: &Db) -> Result<()> {
        Relation::remove(self, other, db)
    }

    #[doc(hidden)]
    fn remove_relation_with_key<E: Entity>(&self, other: &[u8], db: &Db) -> Result<()> {
        Relation::remove_by_keys::<Self, E>(&self.get_key().as_bytes(), other, db)
    }

    /// Gets all entities related to this one in another store.
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let related_struct2s = m_struct_1.get_related::<MyStruct2>(&db)?;
    /// ```
    fn get_related<E: Entity>(&self, db: &Db) -> Result<Vec<E>> {
        Relation::get::<Self, E>(self, db)
    }

    /// Gets all the entities related to this one in another store with a given relation name
    fn get_related_with_name<E:Entity>(&self, name : &str, db:&Db) -> Result<Vec<E>> {
        Relation::get_with_name::<Self,E>(self, name, db)
    }

    /// Gets the first entity related to this one in another store.
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let m_struct_2 = m_struct_1.get_single_related::<MyStruct2>(&db)?;
    /// ```
    fn get_single_related<E: Entity>(&self, db: &Db) -> Result<Option<E>> {
        Relation::get_one::<Self, E>(self, db)
    }

    /// Gets the first entity related to this one in another store with a given relation name
    fn get_single_related_with_name<E: Entity>(&self, name : &str, db: &Db) -> Result<Option<E>> {
        Relation::get_one_with_name::<Self, E>(self,name, db)
    }

    /// Saves `sibling` in its own store after having changed its key to match `self`
    /// This is a convenience method.
    /// 
    /// ⚠ Note that for sibling relations to be fully functionnal, [`get_sibling_trees`](entity/trait.Entity.html#method.get_sibling_trees) must be
    /// overriden
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let m_struct_2 = MyStruct2 { key : 0, prop9 : 32};
    /// m_struct_1.save_sibling(m_struct_2,&db)?;
    /// ```
    fn save_sibling<E: Entity<Key = Self::Key>>(
        &self,
        sibling: &mut E,
        db: &Db,
    ) -> Result<()> {
        sibling.set_key(self.get_key());
        sibling.save(db)
    }

    /// Gets an Entity in another store with the same key as `self`
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let m_struct_2 = m_struct_1.get_sibling::<MyStruct2>(&db)?;
    /// ```
    fn get_sibling<E: Entity<Key = Self::Key>>(&self, db: &Db) -> Result<Option<E>> {
        E::get(&self.get_key(), db)
    }

    /// Saves `child` in its own store after having changed its key to make it effectively a child of `self`
    /// `child` must be an Entity with a Key being the tuple `(Self::Key,u32)` (`Self::Key` being the key type of the parent entity)
    /// 
    /// ⚠ Note that for child relations to be fully functionnal, [`get_child_trees`](entity/trait.Entity.html#method.get_child_trees) must be
    /// overriden
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let m_struct_2 = MyStruct2 { key : (0,0), prop9 : 44};
    /// m_struct1.save_child(m_struct2,&db)?;
    /// ```
    fn save_child<E: Entity<Key = (Self::Key, u32)>>(
        &self,
        child: &mut E,
        db: &Db,
    ) -> Result<E::Key> {
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

    /// Reparents a child to this entity and saves the result to the database.
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let m_struct_2 = MyStruct2::get(&(7,2),&db)?;
    /// m_struct1.adopt_child(m_struct2,&db)?;
    /// ```
    /// After this code, m_struct_2 now has key (9,2) instead of (7,2) and has changed 
    /// accordingly in the database.
    fn adopt_child<E : Entity<Key = (Self::Key,u32)>>(&self, child : &mut E, db : &Db) -> Result<()> {
        E::remove(child.get_key(), db)?;
        self.save_child(child, db)?;
        Ok(())
    }

    /// Gets children Entities from another store
    /// 
    /// ### Exemple 
    /// ```rust
    /// let m_struct_1 = MyStruct1::get(&9,&db)?;
    /// let children = m_struct_1.get_children::<MyStruct2>(&db)?;
    /// ```
    fn get_children<E: Entity<Key = (Self::Key, u32)>>(&self, db: &Db) -> Result<Vec<E>> {
        E::get_with_prefix(self.get_key(), db)
    }

}

/// `AutoIncrementEntity` is a trait aimed to automatically be 
/// implemented on Entities that have `u32` as their `Key` type.
/// 
/// It provides the `save_next()` method that updates the key of the entity 
/// with a new, incremented one before saving it to the database.
pub trait AutoIncrementEntity: Entity<Key = u32> {

    /// Returns a new key that is currently not used in the store
    fn get_next_key(db: &Db) -> Result<u32>;

    /// Saves the entity to the database after having modified its key to an auto-incremented one.
    /// ### Example
    /// ```rust
    /// let m_struct = MyStruct { key : 0, prop9 : 44};
    /// m_struct.save_next(&db)?; // will have key 0
    /// let m_struct_2 = MyStruct { key : 0, prop9 :59};
    /// m_struct_2.save_next(&db)?; // creates a new entry with key 1, and so on
    /// // m_struct2.key is now 1
    /// ```
    fn save_next(&mut self, db: &Db) -> Result<u32>;
}

impl<T> AutoIncrementEntity for T
where
    T: Entity<Key = u32>,
{
    fn get_next_key(db: &Db) -> Result<u32> {
        match Self::get_tree(db)?.last()? {
            Some((key, _)) => Ok(u32::from_be_bytes(key.as_ref().try_into().unwrap()) + 1),
            None => Ok(Default::default()),
        }
    }

    fn save_next(&mut self, db: &Db) -> Result<u32> {
        let next_key = Self::get_next_key(db)?;
        self.set_key(&next_key);
        self.save(db)?;
        Ok(next_key)
    }
}


/// Trait allowing values to be converted to `Vec<u8>`.
/// This trait is not meant to be implemented, but you can if you need to.
pub trait AsBytes {

    /// Returns a new binary representation of `self` as a `Vec<u8>`
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
