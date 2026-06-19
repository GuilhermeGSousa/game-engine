use joltc_sys::JPC_BodyID;

/// A handle to a body living inside the Jolt physics system.
///
/// This is a plain `u32`-backed value (Jolt's `JPC_BodyID`), so it is trivially
/// `Copy` and `Send + Sync` — no raw pointers are stored in the ECS components.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BodyId(pub JPC_BodyID);
