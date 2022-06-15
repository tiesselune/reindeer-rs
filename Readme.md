# Reindeer 🦌

*Reindeer 🦌 lifts your `sled`!*

## A small structural layer on top of `sled`, `serde` and `bincode`

Reindeer is a small entity store built on top of [`sled`](https://sled.rs/), using serde and bincode for serialization, written entirely in rust.

It serves as a convenient middle ground to store, retreive and update structs in an embedded database with a bare-minimum relationnal model.

## Getting Started

### Create a `sled` database

```rs
use reindeer::Db;
```

```rs
let db = reindeer::open("./")?;
```

:bulb: Since this is just a `sled` DB, this object can be copied and sent accross threads safely.

### Implement the `Entity` trait on your `struct`

Entities need to implement the `Serialize` and `Deserialize` traits from `serde`, which are conveniently re-exported from `reindeer`:

```rs
use reindeer::{Serialize,Deserialize,Entity}

#[derive(Serialize,Deserialize)]
pub struct MyStruct {
    pub id : u32,
    pub prop1 : String,
    pub prop2 : u64,
}
```

Then you need to implement the `Entity` trait and implement three methods : `get_key`, `set_key` and `store_name`, as well as define an associated type, `Key`

 - `Key` is the type of the identifier for each instance of your entity ("primary key"). It must implement the `AsByte` trait. 
 😌☝ It's already implemented for `String`, `u32`, `i32`, `u64`, `i64` and `Vec<u8>`, as well as for any 2-elements tuple of those types, so you should not need to implement it yourself.

 - The key represents the unique key that will be used to identify each instance of your struct in the database, to retreive and update them, it is of type `Key`
 - The `store_name` is the name of the entity store. It should be unique for each Entity type (see it as the table name).

```rs
impl Entity for MyStruct{
    type Key = u32;
    fn store_name() -> &'static str {
        "my-struct"
    }
    fn get_key(&self) -> Self::Key {
        self.id
    }
    fn set_key(&mut self, key : Self::Key) {
        self.id = key;
    }
}
```

### Register your entity with the system

Register the entity once, when you launch your application.

```rs
let db = reindeer::open("./")?;
```
```rs
MyStruct::register(db)?;
```

:bulb: Registering the entity will make it possible for Reindeer to handle safe deletion of entity entries. Without this, trying to delete an unregistered entity entry will result in an error.

### Save an instance to the database

You can now save an instance of your struct `MyStruct` to the database :
```rs
let db = reindeer::open("./")?;
```
```rs
let instance = MyStruct {
    id : 0,
    prop1 : String::from("Hello"),
    prop2 : 2335,
}
instance.save(&db)?;
```

:bulb: If `id` 0 already exists in the database, it will be overwritten!

### Retreive an instance from the database

```rs
let instance = MyStruct::get(0,&db)?;
```

### Retreive all instances

```rs
let instances = MyStruct::get_all(&db)?;

```

### Get All entities respecting a condition


```rs
let instances = MyStruct::get_with_filter(|m_struct| {mstruct.prop1.len > 20},&db)?;
```

### Delete an instance from the database

```rs
MyStruct::remove(0,&db)?;
```

## Defining Relations

`reindeer` has three types of relations : 

 - `sibling` : An entity that has the same key in another tree (one to one relation)
 - `parent-child` : An entity which key is composed of its parent's key and a `u32` (as a two-element tuple) for efficient one-to-many relations
 - `free-relation` you freely connect two instances of two separate Entities together. This can be used to achieve many-to-many relationships, but is less efficient than sibling and parent-child relationships in regard to querying the database. **Use when sibling and parent-child are not possible.**

### Sibling relationships

To create a sibling Entity, you need to link the Entity structs together by overriding the `get_sibling_trees()` method :

```rs
use reindeer::{Entity,DeletionBehaviour};
impl Entity for MyStruct1{
    /* ... */
    fn store_name() -> &'static str {
        "my_struct_1"
    }
    fn get_sibling_trees() -> Vec<(&'static str,DeletionBehaviour)> {
        return vec![("my_struct_2",DeletionBehaviour::Cascade)]
    }
}

impl Entity for MyStruct2{
    /* ... */
    fn store_name() -> &'static str {
        "my_struct_2"
    }
    fn get_sibling_trees() -> Vec<(&'static str,DeletionBehaviour)> {
        return vec![("my_struct_1",DeletionBehaviour::BreakLink)]
    }
}
```

:bulb: if sibling trees are defined, an entity instance might or might not have a sibling of the other Sibling type! Siblings are optionnal by default.

:bulb: DeletionBehaviour determines what happens to the sibbling when the current entity is removed :

 - `DeletionBehaviour::Cascade` also deletes sibling entity
 - `DeletionBehaviour::Error` causes an Error if a sibling still exists and does not delete the source element
 - `DeletionBehaviour::BreakLink` just removes the entity without removing its sibling.

In the above example, deleting a `MyStruct1` instance also deletes its sibling `MyStruct2` instance, but deleting the `MyStruct2` instance leaves its sibling `MyStruct1` instance intact.

:bulb: Sibling Entities must have the same `Key` type.

#### Creating a sibling entity

```rs
let m_struct_1 = MyStruct1 {
    /* ... */
};
let mut m_struct_2 = MyStruct2 {
    /* ... */
};
m_struct_1.save(&db)?;
m_struct_1.save_sibling(m_struct_2,&db)?;
```

:bulb: this will update `m_struct_2`'s key to `m_struct_1`'s key using the `set_key` method, so it does not matter which key you initially provide before calling `save_child`.

:warning: Note that if you create an entity in `MyStruct2`'s store with the same key as an entity in `MyStruct1`'s store without using `save_sibling`, the result is the same, and the two entities will be considered siblings all the same.

#### Retrieving a sibling entity

```rs
if let Some(sibling) = m_struct_1.get_sibling::<MyStruct2>(&db)? {
    /* ... */
}
```

:bulb: Note that a sibling may or may not be present, thus the `Option` type.

### Parent-child relationship

For a parent-child relationship between entities to exist, the child entity must have a `Key` type being a tuple of :
 - The parent `Key` type
 - `u32`

:bulb: Children entities will be auto-incremeted and easily retreived through their parent key.


```rs
impl Entity for Parent{
    type Key = String;
    /* ... */
    fn store_name() -> &'static str {
        "parent"
    }
    fn get_child_trees() -> Vec<(&'static str)> {
        return vec![("child", DeletionBehaviour::Cascade)]
    }
}

impl Entity for Child{
    type Key = (String, u32);
    /* ... */
    fn store_name() -> &'static str {
        "child"
    }
}
```

In the above example, deleting the parent entity will remove all child entities automatically (thanks to the `Cascade` deletion behaviour).
**For database integrity, it is strongly advised not to use `DeletionBehaviour::BreakLink` on parent/child relations,** and instead use either `Error` of `Cascade`

#### Adding a child entity

```rs
let parent = Parent {
    /* ... */
};

let mut child = Child {
    /* ... */
}

parent.save_child(child,&db)?;
```

:bulb: this will update `child`'s key to `parent`'s key and an auto-incremented index using the `set_key` method, so it does not matter which key you initially provide before calling `save_child`.

#### Getting Children

```rs
let children = parent.get_children::<Child>(&db)?;
```

### Free relations

Free relations follow the same pattern as other relation types, except they are freely created between any two entities. This can be used to achieve many to many relationships.

:bulb: Creating a free relation will automatically create its opposite relation, making it two-way.

#### Linking two entities 

```rs
let e1 = Entity1 {
    /* ... */
};

let mut e2 = Entity2 {
    /* ... */
}

e1.create_relation(e2,DeletionBehaviour::Cascade, DeletionBehaviour::BreakLink,&db)?;
```

In the above example, deletion behaviour in both ways are provided : deleting `e1` will automatically delete `e2`, but deleting `e2` will leave `e1` untouched and break the link between them.

`DeletionBehaviour::Error` is also an option here.

#### Getting related entites from a given tree

```rs
let related_entities = e1.get_related::<Entity2>(db)?;
```
To get only the first related entity from the other tree, use 

```rs
let related_entity = e1.get_single_related::<Entity2>(db)?;
```

#### Breaking a free relation link

If needed, you can remove an existing link between entities:

```rs
e1.remove_relation(other,db)?;
```
or
```rs
e1.remove_relation_with_key::<OtherEntity>(otherKey,db)?;
```

### Deadlocks 🔒

When defining `DeletionBehaviour` for your relations, be careful **not to create deadlocks**.

For instance, if two siblings mutually define a `DeletionBehaviour::Error` link, then none of them can ever be removed...

Also, be aware of the cycles you create in databases. While you can create relation cycles safely, the same deadlock rules as above apply, and the library will not detect them until you try to delete something.

### Performance

While Sibling and Parent-child relations are performant by default, Free relations are less performant and rely on hidden object stores to work, forcing reads and writes to the database on relation creation and entity deletion. Be aware of this pitfall.

Also, defining cascading relations will run through relations reccursively when deleting entities, making the operation heavier than relation-less entities.

### Auto-incrementing entities

If your entity `Key` type is `u32`, you can auto-increment new entities using

```rs
use reindeer::AutoIncrementEntity;
let mut new_entity = Entity {
    id : 0 // if you setup id with any key, saving will update it
    /* ... */
};
new_entity.save_next(db)?;
// new_entity's key is now the auto-incremente value
```

You entitie's key will be automatically updated with `set_key` to match the last found entry's ID, incremented by 1.

:bulb: Note that the `AutoIncrementEntity` trait needs to be in scope.