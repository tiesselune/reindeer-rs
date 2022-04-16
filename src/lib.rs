pub mod entity;
#[macro_use] mod macros;
pub use entity::Entity;
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize,Deserialize)]
pub struct MyEntity {
    pub id : String,
    pub content : String,
}

impl Entity for MyEntity {
    type Key = String;

    fn tree_name() -> &'static str {
        "somename"
    }

    fn get_key(&self) -> Self::Key {
        self.id.clone()
    }
}

entity!(MyEntity2,u32,{
    content : String,
},{
    parent : MyEntity, One,
});


#[cfg(test)]
mod tests {
    use crate::MyEntity2;

    #[test]
    fn it_works() {
        let result = MyEntity2 { key: todo!(), content: todo!(), parent_opt: todo!(), parent_id: todo!() };
    }
}
