pub mod entity;
pub mod relation;
pub use entity::Entity;
pub use relation::{RelationMany, RelationSingle};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
