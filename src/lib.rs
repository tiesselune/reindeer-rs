pub mod entity;
pub mod relation;
pub use entity::AutoIncrementEntity;
pub use entity::Entity;
pub use relation::DeletionBehaviour;
pub use serde_derive::{Deserialize, Serialize};
pub use sled::open;
pub use sled::Db;

#[cfg(test)]
mod test;
