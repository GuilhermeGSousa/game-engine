use std::{any::Any, hash::Hash};

pub trait DynEq: Any {
    fn dyn_eq(&self, other: &dyn DynEq) -> bool;
}

impl<T> DynEq for T
where
    T: Any + Eq,
{
    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        (other as &dyn Any)
            .downcast_ref::<T>()
            .is_some_and(|val| self == val)
    }
}

pub trait DynHash: DynEq {
    fn dyn_hash(&self, hasher: &mut dyn std::hash::Hasher);
}

impl<T> DynHash for T
where
    T: DynEq + Hash,
{
    fn dyn_hash(&self, mut hasher: &mut dyn std::hash::Hasher) {
        T::hash(&self, &mut hasher);
        self.type_id().hash(&mut hasher);
    }
}

#[macro_export]
macro_rules! define_label {
    (
        $(#[$label_attr:meta])*
        $label_trait_name:ident
    ) => {
        $(#[$label_attr])*
        pub trait $label_trait_name:
            Send + Sync + $crate::label::DynHash + $crate::label::DynEq
        {
            fn dyn_clone(&self) -> Box<dyn $label_trait_name>;
        }

        impl ::std::hash::Hash for dyn $label_trait_name + 'static {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.dyn_hash(state);
            }
        }

        impl ::std::cmp::PartialEq for dyn $label_trait_name {
            fn eq(&self, other: &Self) -> bool {
                self.dyn_eq(other)
            }
        }

        impl ::std::cmp::Eq for dyn $label_trait_name {}
    };
}
