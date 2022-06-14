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
fn test_save_save_next_and_get() -> Result<(), std::io::Error> {
    let name = Uuid::new_v4().to_string();
    let db = set_up(&name)?;
    set_up_content(&db)?;
    let e1_0 = Entity1::get(&0,&db)?;
    let e1_1 = Entity1::get(&1,&db)?;
    let e2_1 = Entity2::get(&String::from("id1"),&db)?;
    let e2_2 = Entity2::get(&String::from("id2"),&db)?;
    assert!(e1_0.is_some());
    assert!(e1_1.is_some());
    assert!(e2_1.is_some());
    assert!(e2_2.is_some());
    let e1_0 = e1_0.unwrap();
    let e1_1 = e1_1.unwrap();
    let e2_1 = e2_1.unwrap();
    let e2_2 = e2_2.unwrap();
    assert_eq!(e1_0.id,0);
    assert_eq!(e1_0.prop1,"Hello, World!");
    assert_eq!(e1_1.id,1);
    assert_eq!(e1_1.prop1,"Hello, Nancy!");
    assert!(Entity1::get(&8,&db)?.is_none());
    assert_eq!(e2_1.prop2,3);
    assert_eq!(e2_2.prop2,5);
    tear_down(&name)?;
    Ok(())
}

#[test]
fn test_save_and_get_children() -> Result<(), std::io::Error> {
    let name = Uuid::new_v4().to_string();
    let db = set_up(&name)?;
    set_up_content(&db)?;
    let child_1 = ChildEntity1::get(&(String::from("id3"),0), &db)?;
    assert!(child_1.is_some());
    let e2_3 = Entity2::get(&String::from("id3"),&db)?.unwrap();
    let children = e2_3.get_children::<ChildEntity1>(&db)?;
    assert_eq!(children.len(),3);
    tear_down(&name)?;
    Ok(())
}


#[test]
fn test_cascade_children() -> Result<(), std::io::Error> {
    let name = Uuid::new_v4().to_string();
    let db = set_up(&name)?;
    set_up_content(&db)?;
    let e2_3 = Entity2::get(&String::from("id3"),&db)?.unwrap();
    let children = e2_3.get_children::<ChildEntity1>(&db)?;
    assert_eq!(children.len(),3);
    Entity2::remove(&String::from("id3"), &db)?;
    assert!(Entity2::get(&String::from("id3"),&db)?.is_none());
    assert_eq!(e2_3.get_children::<ChildEntity1>(&db)?.len(),0);
    tear_down(&name)?;
    Ok(())
}

#[test]
fn test_delete_children_error() -> Result<(), std::io::Error> {
    let name = Uuid::new_v4().to_string();
    let db = set_up(&name)?;
    set_up_content(&db)?;
    let e3_2 = Entity3::get(&2,&db)?.unwrap();
    let children = e3_2.get_children::<ChildEntity2>(&db)?;
    assert_eq!(children.len(),3);
    assert!(Entity3::remove(&2, &db).is_err());
    let e3_2 = Entity3::get(&2,&db)?;
    assert!(e3_2.is_some());
    assert_eq!(e3_2.unwrap().get_children::<ChildEntity2>(&db)?.len(),3);
    tear_down(&name)?;
    Ok(())
}