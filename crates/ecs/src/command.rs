use crate::{component::{bundle::ComponentBundle, Component}, entity::{entity_store::EntityStore, Entity},
    system::system_input::SystemInput, world::World,
};

pub struct CommandQueue<'world, 'state> {
    queue_state: &'state mut CommandQueueState,
    entities: &'world mut EntityStore,
}

impl<'w, 's> CommandQueue<'w, 's> {
    pub(crate) fn new(state: &'s mut CommandQueueState, entities: &'w mut EntityStore) -> Self {
        Self {
            queue_state: state,
            entities,
        }
    }

    pub fn spawn<T: ComponentBundle + 'static>(&mut self, components: T) -> Entity {
        let entity = self.entities.alloc();
        self.queue_state
            .add_command(SpawnCommand::new(components, entity));
        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.queue_state.add_command(DespawnCommand::new(entity));
    }

    pub fn insert<T: Component>(&mut self, component: T, entity: Entity) {
        self.queue_state
            .add_command(InsertCommand::new(component, entity));
    }
}

pub struct CommandQueueState {
    queue: Vec<Box<dyn Command>>,
}

impl CommandQueueState {
    pub fn new() -> Self {
        CommandQueueState { queue: Vec::new() }
    }

    pub fn add_command<C: Command + 'static>(&mut self, command: C) {
        self.queue.push(Box::new(command));
    }

    pub fn execute_commands(&mut self, world: &mut World) {
        for command in self.queue.drain(..) {
            command.execute(world);
        }
    }
}

unsafe impl SystemInput for CommandQueue<'_, '_> {
    type State = CommandQueueState;
    type Data<'world, 'state> = CommandQueue<'world, 'state>;

    fn init_state() -> Self::State {
        CommandQueueState::new()
    }

    unsafe fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: crate::world::UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        CommandQueue::new(state, world.world_mut().get_entity_store_mut())
    }

    fn apply(state: &mut Self::State, world: &mut World) {
        state.execute_commands(world);
    }
}

pub trait Command {
    fn execute(self: Box<Self>, world: &mut World);
}

pub(crate) struct SpawnCommand<T: ComponentBundle> {
    components: T,
    entity: Entity,
}

impl<T: ComponentBundle> SpawnCommand<T> {
    pub fn new(components: T, entity: Entity) -> Self {
        SpawnCommand { components, entity }
    }
}

impl<T: ComponentBundle> Command for SpawnCommand<T> {
    fn execute(self: Box<Self>, world: &mut World) {
        world.spawn_allocated(self.entity, self.components);
    }
}

pub(crate) struct DespawnCommand {
    entity: Entity,
}

impl DespawnCommand {
    pub fn new(entity: Entity) -> Self {
        DespawnCommand { entity }
    }
}

impl Command for DespawnCommand {
    fn execute(self: Box<Self>, world: &mut World) {
        world.despawn(self.entity);
    }
}

pub(crate) struct InsertCommand<T: Component> {
    component: T,
    entity: Entity,
}

impl<T: Component> InsertCommand<T> {
    pub fn new(component: T, entity: Entity) -> Self {
        InsertCommand { component, entity }
    }
}

impl<T: Component> Command for InsertCommand<T> {
    fn execute(self: Box<Self>, world: &mut World) {
        world.insert_component(self.component, self.entity);
    }
}
