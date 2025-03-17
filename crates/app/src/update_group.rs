#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    Update,
    LateUpdate,
    Render,
}
