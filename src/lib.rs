pub mod entity;
pub mod relation;
pub use entity::Entity;
pub use relation::{RelationSingle,RelationMany};


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        
    }
}
