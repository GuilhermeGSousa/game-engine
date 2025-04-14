#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    Startup,
    Update,
    LateUpdate,
    Render,
}
