pub mod entity;
pub mod relation;
pub use entity::Entity;
pub use entity::AutoIncrementEntity;
pub use sled::Db;
pub use sled::open;
pub use serde_derive::{Serialize,Deserialize};
pub use relation::DeletionBehaviour;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
