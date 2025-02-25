#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    Update,
    FixedUpdate,
    PostUpdate,
    Last,
}
