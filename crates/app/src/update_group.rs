#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    Startup,
    Update,
    FixedUpdate,
    LateUpdate,
    LateFixedUpdate,
    Render,
    LateRender,
}
