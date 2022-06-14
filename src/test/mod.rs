mod test_entities;

use test_entities::{Entity1,Entity2,Entity3,Entity4,ChildEntity1,ChildEntity2,set_up,set_up_content,tear_down};
use crate::{open, relation::FamilyDescriptor, Entity};
use uuid::Uuid;

#[test]
fn create_and_register() -> Result<(), std::io::Error> {
    let name = Uuid::new_v4().to_string();
    let db = set_up(&name)?;
    assert!(FamilyDescriptor::exists(&String::from("entity_1"), &db)?);
    assert!(FamilyDescriptor::exists(&String::from("entity_2"), &db)?);
    assert!(FamilyDescriptor::exists(&String::from("child_entity_1"), &db)?);
    let fam_desc = FamilyDescriptor::get(&String::from("entity_1"),&db)?;
    assert!(fam_desc.is_some());
    assert_eq!(fam_desc.unwrap().sibling_trees.len(),1);
    tear_down(&name)?;
    Ok(())
}

#[test]
fn test_insert_and_get() -> Result<(), std::io::Error> {
    let name = Uuid::new_v4().to_string();
    let db = set_up(&name)?;
    set_up_content(&db)?;
    let e1_0 = Entity1::get(&0,&db)?;
    let e1_1 = Entity1::get(&1,&db)?;
    assert!(e1_0.is_some());
    assert!(e1_1.is_some());
    let e1_0 = e1_0.unwrap();
    let e1_1 = e1_1.unwrap();
    assert_eq!(e1_0.id,0);
    assert_eq!(e1_0.prop1,"Hello, World!");
    assert_eq!(e1_1.id,1);
    assert_eq!(e1_1.prop1,"Hello, Nancy!");
    assert!(Entity1::get(&8,&db)?.is_none());
    tear_down(&name);
    Ok(())
}