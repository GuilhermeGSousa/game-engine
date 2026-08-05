use std::collections::HashMap;

use ecs::World;

#[derive(Default)]
pub struct SubApps {
    main: SubApp,
}

#[derive(Default)]
pub struct SubApp {
    world: World,
}
