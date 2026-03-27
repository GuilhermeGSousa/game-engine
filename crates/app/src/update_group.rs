/// Identifies which phase of the per-frame update loop a system belongs to.
///
/// Systems within the same group run in insertion order.  Groups themselves run
/// in this fixed order each frame (driven by [`App::update`](crate::App::update)):
///
/// 1. **Startup** — runs once when [`App::finish_plugin_build`](crate::App::finish_plugin_build) is called.
/// 2. **FixedUpdate** — runs zero or more times per frame to catch up with a fixed time step.
/// 3. **LateFixedUpdate** — runs once after each FixedUpdate pass.
/// 4. **Update** — the main per-frame update phase.
/// 5. **LateUpdate** — cleanup/reaction phase (e.g. event flushing, transform propagation).
/// 6. **Render** — submits draw calls to the GPU.
/// 7. **LateRender** — post-render work (e.g. UI overlay).
#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    /// One-shot startup systems, run before the first frame.
    Startup,
    /// Main per-frame update.
    Update,
    /// Fixed-timestep physics/logic update.
    FixedUpdate,
    /// Runs after `Update` (e.g. transform propagation, event flushing).
    LateUpdate,
    /// Runs after each `FixedUpdate` pass.
    LateFixedUpdate,
    /// GPU render submission.
    Render,
    /// Post-render overlay (e.g. UI).
    LateRender,
}
