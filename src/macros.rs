
#[macro_export]
macro_rules! entity {
    ($name:ident, $key_type:ty,{ $($field_name:ident : $field_type:ty,)*},{ $($relation_name:ident : $relation_type:ty, One,)* }) => {

        paste::paste! {
            #[derive(Serialize,Deserialize)]
            pub struct $name {
                pub key : $key_type,
                $(pub $field_name : $field_type,)*
                $(
                    #[serde(skip_serializing)]
                    pub [<$relation_name _opt>] : Option<$relation_type>,
                )*
                $(pub [<$relation_name _id>] : Option<<$relation_type as Entity>::Key>,)*
            }
        }

        impl Entity for $name {
            type Key = $key_type;
            fn tree_name() -> &'static str {
                stringify!($name)
            }
            fn get_key(&self) -> Self::Key {
                self.key.clone()
            }
        }
    }
}
