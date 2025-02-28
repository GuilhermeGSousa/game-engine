use crate::{query::ComponentQuery, world::World};

pub trait SystemFunction<Args> {
    fn run(&self, world: &mut World);
}

// Practice here!
impl<FunctionType, A> SystemFunction<(A,)> for FunctionType
where
    FunctionType: Fn(A) + 'static,
    A: ComponentQuery + 'static,
{
    fn run(&self, world: &mut World) {
        for archetype in world.get_archetypes_mut() {
            todo!()
        }
    }
}

impl<FunctionType, A, B> SystemFunction<(A, B)> for FunctionType
where
    FunctionType: Fn(A, B) + 'static,
    A: ComponentQuery + 'static,
    B: ComponentQuery + 'static,
{
    fn run(&self, world: &mut World) {
        for archetype in world.get_archetypes_mut() {
            let has_all_components =
                archetype.has_component::<A>() && archetype.has_component::<B>();

            if !has_all_components {
                continue;
            }

            let len = archetype.len();
            unsafe {
                for i in 0..len {
                    let a = A::get_component_unsafe(archetype, i);
                    let b = B::get_component_unsafe(archetype, i);
                    self(a, b);
                }
            }
        }
    }
}

macro_rules! impl_system_function {
    ($($arg:ident),*) => {
        impl<FunctionType, $($arg),*> SystemFunction<($($arg,)*)> for FunctionType
        where
            FunctionType: Fn($($arg),*) + 'static,
            $($arg: ComponentQuery),*
        {
            fn run(&self, world: &mut World){}
        }
    };
}

//impl_system_function!(A);
//impl_system_function!(A, B);
impl_system_function!(A, B, C);
impl_system_function!(A, B, C, D);
impl_system_function!(A, B, C, D, E);
impl_system_function!(A, B, C, D, E, F);
impl_system_function!(A, B, C, D, E, F, G);
impl_system_function!(A, B, C, D, E, F, G, H);
impl_system_function!(A, B, C, D, E, F, G, H, I);
impl_system_function!(A, B, C, D, E, F, G, H, I, J);
impl_system_function!(A, B, C, D, E, F, G, H, I, J, K);
impl_system_function!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_system_function!(A, B, C, D, E, F, G, H, I, J, K, L, M);
