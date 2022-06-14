# Sled Entity

## A small structural layer on top of `sled`, `serde` and `bincode`

Sled Entity is a small structural layer on top of sled, using serde and bincode for serialization, written entirely in pure rust.

It serves as a convenient middle ground to store, retreive and update structs in an embedded database with a bare-minimum relationnal model.

## Getting Started

### Create a sled database

```rs
use sled_entity::Db;
```

```rs
let db = sled_entity::open("./")?;
```

### Implement the `Entity` trait on your `struct`

Entities need to implement the `Serialize` and `Deserialize` traits from `serde`, which are conveniently re-exported from `sled_entity`:

```rs
use sled_entity::{Serialize,Deserialize,Entity}

#[derive(Serialize,Deserialize)]
pub struct MyStruct {
    pub id : u32,
    pub prop1 : String,
    pub prop2 : u64,
}
```

Then you need to implement the `Entity` trait and implement three methods : `get_key`, `set_key` and `tree_name`, as well as define an associated type, `Key`

 - `Key` is the type of the identifier for each instance of your entity. It must implement the `AsByte` trait. 
 😌☝ It's already implemented for `String`, `u32`, `i32`, `u64`, `i64` and `Vec<u8>`, as well as for any 2-elements tuple of those types, so you should not need to implement it yourself.

 - The `key` represents the unique key that will be used to identify each instance of your struct in the database, to retreive and update them, it is of type `Key`
 - The `tree_name` is the name of the store. It should be unique for each Entity type.

```rs
impl Entity for MyStruct{
    type Key = u32;
    fn tree_name() -> &'static str {
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

### Save an instance to the database

You can now save an instance of your struct `MyStruct` to the database :
```rs
let db = sled_entity::open("./")?;
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

`sled_entity` has three types of relations : 

 - `sibling` : An entity that has the same key in another tree (one to one relation)
 - `parent-child` : An entity which key is composed of its parent's key and a `u32` (as a two-element tuple) (one to many relation)
    - Child entities cannot live without a parent entity
    - Deleting the parent entity will delete child entities automatically
 - `free-relation` you freely connect two instances of two separate Entities together. The relationship must be broken to be able to remove any of the two. This can be used to achieve many to many relationships, but is less efficient than sibling and parent-child relationships in regard to querying the database.

### Sibling relationships

To create a sibling Entity, you need to link the Entity structs together by overriding the `get_sibling_trees()` method :

```rs
impl Entity for MyStruct1{
    /* ... */
    fn tree_name() -> &'static str {
        "my_struct_1"
    }
    fn get_sibling_trees() -> Vec<(&'static str,DeletionBehaviour)> {
        return vec![("my_struct_1",DeletionBehaviour::Cascade)]
    }
}

impl Entity for MyStruct2{
    /* ... */
    fn tree_name() -> &'static str {
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
 - `DeletionBehaviour::Error` causes an Error if a sibling still exists
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

:bulb: this will update `m_struct_2`'s key to `m_struct_1`'s key using the `set_key` method.

#### Retrieving a sibling entity

```rs
if let Some(sibling) = m_struct_1.get_sibling::<MyStruct2>(&db)? {
    /* ... */
}
```

### Parent-child relationship

For a parent-child relationship between entities to exist, the child entity must have a `Key` type being a tuple of :
 - The parent `Key` type
 - `u32`

Children entities will be auto-incremeted and easily retreived through their parent key.

Children entities will be removed if parent entity is removed.

```rs
impl Entity for Parent{
    type Key = String;
    /* ... */
    fn tree_name() -> &'static str {
        "parent"
    }
    fn get_child_trees() -> Vec<(&'static str)> {
        return vec!["child"]
    }
}

impl Entity for Child{
    type Key = (String, u32);
    /* ... */
    fn tree_name() -> &'static str {
        "child"
    }
}
```

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

:bulb: this will update `child`'s key to `parent`'s key and an auto-incremented index using the `set_key` method.

#### Getting Children

```rs
let children = parent.get_children::<Child>(&db)?;
```

### Free relations

WIP

### Auto-incrementing entities

WIP
