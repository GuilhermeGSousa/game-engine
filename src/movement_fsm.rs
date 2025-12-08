use statig::{
    state_machine,
    Outcome::{self, Super, Transition},
};

#[derive(Debug, Default)]
pub struct MovementFSM;

#[derive(Debug)]
pub enum MovementEvent {
    StickInputReceived,
    StickInputReleased,
}

#[state_machine(
    // This sets the initial state to `led_on`.
    initial = "State::idle()",
)]
impl MovementFSM {
    #[state]
    fn idle(event: &MovementEvent) -> Outcome<State> {
        match event {
            MovementEvent::StickInputReceived => Transition(State::moving()),
            _ => Super,
        }
    }

    #[state]
    fn moving(event: &MovementEvent) -> Outcome<State> {
        match event {
            MovementEvent::StickInputReleased => Transition(State::idle()),
            _ => Super,
        }
    }
}
