pub mod collider;
pub mod physics_pipeline;
pub mod physics_server;
pub mod physics_state;
pub mod plugin;
pub mod rigid_body;
mod simulation;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
