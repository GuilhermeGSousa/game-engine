pub mod event_channel;
pub mod event_reader;
pub mod event_writer;

pub use ecs_macros::Event;

pub trait Event {}
