use std::marker::PhantomData;

use crate::{
    component::{bundle::ComponentBundle, Component},
    entity::{entity_store::EntityStore, Entity},
    resource::Resource,
    system::system_input::SystemInput,
    world::World,
};

pub struct EntityCommandQueue<'a> {
    entity: Entity,
    command_queue: CommandQueue<'a, 'a>,
}

impl<'a> EntityCommandQueue<'a> {
    pub fn entity(&self) -> &Entity {
        &self.entity
    }

    pub fn add_child<T: ComponentBundle + 'static>(&mut self, components: T) -> &mut Self {
        self.add_child_with(components, |_| {})
    }

    pub fn add_child_with<T: ComponentBundle + 'static>(
        &mut self,
        components: T,
        f: impl Fn(EntityCommandQueue),
    ) -> &mut Self {
        let child_ctx = self.command_queue.spawn(components);
        let child_entity = *child_ctx.entity();
        f(child_ctx);
        self.command_queue.add_child(self.entity, child_entity);
        self
    }

    pub fn insert<T: Component>(&mut self, component: T) {
        self.command_queue.insert(component, self.entity);
    }

    pub fn despawn(mut self) {
        self.command_queue.despawn(*self.entity());
    }
}

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

    pub fn spawn<T: ComponentBundle + 'static>(&mut self, components: T) -> EntityCommandQueue<'_> {
        let spawned_entity = self.entities.alloc();
        self.queue_state
            .add_command(SpawnCommand::new(components, spawned_entity));

        EntityCommandQueue {
            entity: spawned_entity,
            command_queue: CommandQueue {
                queue_state: &mut *self.queue_state,
                entities: &mut *self.entities,
            },
        }
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.queue_state.add_command(DespawnCommand::new(entity));
    }

    pub fn insert<T: Component>(&mut self, component: T, entity: Entity) {
        self.queue_state
            .add_command(InsertCommand::new(component, entity));
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.queue_state
            .add_command(RemoveCommand::<T>::new(entity));
    }

    pub fn add_child(&mut self, parent: Entity, child: Entity) {
        self.queue_state.add_command(AddChild::new(parent, child));
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.queue_state
            .add_command(InsertResource::<T>::new(resource));
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

impl Default for CommandQueueState {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInput for CommandQueue<'_, '_> {
    type State = CommandQueueState;
    type Data<'world, 'state> = CommandQueue<'world, 'state>;

    fn init_state() -> Self::State {
        CommandQueueState::new()
    }

    fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: crate::world::UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        CommandQueue::new(state, world.world_mut().entity_store_mut())
    }

    fn apply(state: &mut Self::State, world: &mut World) {
        state.execute_commands(world);
    }

    fn fill_access(access: &mut crate::system::access::SystemAccess) {
        access.write_world();
    }
}

pub trait Command: Send + Sync {
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

pub(crate) struct RemoveCommand<T: Component> {
    entity: Entity,
    _marker: PhantomData<T>,
}

impl<T: Component> RemoveCommand<T> {
    pub fn new(entity: Entity) -> Self {
        RemoveCommand {
            entity,
            _marker: PhantomData,
        }
    }
}

impl<T: Component> Command for RemoveCommand<T> {
    fn execute(self: Box<Self>, world: &mut World) {
        world.remove_component::<T>(self.entity);
    }
}

pub(crate) struct AddChild {
    parent: Entity,
    child: Entity,
}

impl AddChild {
    pub fn new(parent: Entity, child: Entity) -> Self {
        Self { parent, child }
    }
}

impl Command for AddChild {
    fn execute(self: Box<Self>, world: &mut World) {
        world.add_child(self.parent, self.child);
    }
}

pub(crate) struct InsertResource<T: Resource> {
    resource: T,
}

impl<T: Resource> InsertResource<T> {
    fn new(resource: T) -> Self {
        Self { resource }
    }
}

impl<T: Resource> Command for InsertResource<T> {
    fn execute(self: Box<Self>, world: &mut World) {
        world.insert_resource(self.resource);
    }
}
