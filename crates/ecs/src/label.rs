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
        T::hash(self, &mut hasher);
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
            Send + Sync + std::fmt::Debug + $crate::label::DynHash + $crate::label::DynEq
        {
            fn dyn_clone(&self) -> Box<dyn $label_trait_name>;

            fn intern(&self) -> $crate::intern::Interned<dyn $label_trait_name>
            where Self: Sized {
                static INTERNER: $crate::intern::Interner<dyn $label_trait_name> =
                    $crate::intern::Interner::new();

                INTERNER.intern(self)
            }
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


        impl $crate::intern::Internable for dyn $label_trait_name {
            fn leak(&self) -> &'static Self {
                Box::leak(self.dyn_clone())
            }

            fn ref_eq(&self, other: &Self) -> bool
            {
                use ::core::ptr;

                self.type_id() == other.type_id()
                    && ptr::addr_eq(ptr::from_ref::<Self>(self), ptr::from_ref::<Self>(other))
            }

            fn ref_hash<H: core::hash::Hasher>(&self, state: &mut H)
            {
                use ::core::{hash::Hash, ptr};

                // Hash the type id...
                self.type_id().hash(state);

                // ...and the pointer address.
                // Cast to a unit `()` first to discard any pointer metadata.
                ptr::from_ref::<Self>(self).cast::<()>().hash(state);
            }
        }

        impl $label_trait_name for Interned<dyn $label_trait_name>
        {
            fn dyn_clone(&self) -> Box<dyn $label_trait_name>
            {
                (**self).dyn_clone()
            }

            fn intern(&self) -> Self {
                *self
            }
        }
    };
}
