use std::any::TypeId;

use crate::component::Component;

pub trait ComponentBundle {
    fn get_type_ids() -> Vec<TypeId>;
}

macro_rules! tuple_impls {
    ( $head:ident, $( $tail:ident, )* ) => {
        impl<$head, $( $tail ),*> ComponentBundle for ($head, $( $tail ),*)
        where
            $head: Component,
            $( $tail: Component ),*
        {
            fn get_type_ids() -> Vec<TypeId> {
                let mut type_ids = Vec::new();
                type_ids.push(TypeId::of::<$head>());
                $( type_ids.push(TypeId::of::<$tail>()); )*
                type_ids.sort();
                type_ids
            }
        }

        tuple_impls!($( $tail, )*);
    };

    () => {};
}

tuple_impls!(A, B, C, D, E, F, G, H, I, J,);
