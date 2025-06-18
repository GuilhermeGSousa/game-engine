#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    Startup,
    Update,
    LateUpdate,
    FixedUpdate,
    LateFixedUpdate,
    Render,
    LateRender,
}
