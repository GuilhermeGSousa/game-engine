use crate::{query::SystemInput, world::World};

pub trait SystemFunction<Args> {
    fn run(&self, world: &mut World);
}

impl<FunctionType, A> SystemFunction<(A,)> for FunctionType
where
    FunctionType: Fn(A) + 'static,
    A: SystemInput + 'static,
{
    fn run(&self, world: &mut World) {
        let archetypes = world.get_archetypes_mut();
        for archetype in archetypes {
            let mut has_all_components = true;
            has_all_components &= archetype.has_component::<A>();
            if !has_all_components {
                continue;
            }
            let len = archetype.len();
            unsafe {
                for i in 0..len {
                    self(A::get_component_data_unsafe(archetype, i));
                }
            }
        }
    }
}

// Try doing this with typle!
macro_rules! impl_system_function {
    ($($arg:ident),*) => {
        impl<FunctionType, $($arg),*> SystemFunction<($($arg,)*)> for FunctionType
        where
            FunctionType: Fn($($arg),*) + 'static,
            $($arg: SystemInput + 'static),*
        {
            fn run(&self, world: &mut World){
                for archetype in world.get_archetypes_mut() {
                    let mut has_all_components = true;
                    $(
                        has_all_components &= archetype.has_component::<$arg>();
                    )*

                    if !has_all_components {
                        continue;
                    }

                    let len = archetype.len();

                    unsafe {
                        for i in 0..len {
                            self(
                                $(
                                    $arg::get_component_data_unsafe(archetype, i),
                                )*
                            );
                        }
                    }
                }
            }
        }
    };
}

// impl_system_function!(A);
impl_system_function!(A, B);
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
