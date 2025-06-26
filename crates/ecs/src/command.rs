use crate::{system::system_input::SystemInput, world::World};

pub struct CommandQueue<'state> {
    queue_state: &'state mut CommandQueueState,
}

impl<'s> CommandQueue<'s> {
    pub fn new(state: &'s mut CommandQueueState) -> Self {
        Self { queue_state: state }
    }

    pub fn execute(&mut self, world: &mut World) {
        self.queue_state.execute_commands(world);
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

unsafe impl SystemInput for CommandQueue<'_> {
    type State = CommandQueueState;
    type Data<'world, 'state> = CommandQueue<'state>;

    fn init_state() -> Self::State {
        CommandQueueState::new()
    }

    unsafe fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        _world: crate::world::UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        CommandQueue::new(state)
    }
}

pub trait Command {
    fn execute(&self, world: &mut World);
}
