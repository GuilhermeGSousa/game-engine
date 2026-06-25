//! Collision-layer configuration for the Jolt physics system.
//!
//! Jolt separates collision filtering into *object layers* (per body) and
//! *broad-phase layers* (coarse buckets used by the broad phase). `Init`
//! requires three filter interfaces describing how those layers map and which
//! pairs are allowed to collide. JoltC exposes these as function-pointer
//! vtables; we provide a fixed two-layer setup (`MOVING` / `NON_MOVING`) that
//! mirrors Jolt's "HelloWorld" example.

use std::ffi::{c_uint, c_void};
use std::ptr;

use joltc_sys::*;

pub const OL_NON_MOVING: JPC_ObjectLayer = 0;
pub const OL_MOVING: JPC_ObjectLayer = 1;

pub const BPL_NON_MOVING: JPC_BroadPhaseLayer = 0;
pub const BPL_MOVING: JPC_BroadPhaseLayer = 1;
pub const BPL_COUNT: JPC_BroadPhaseLayer = 2;

unsafe extern "C" fn bpl_get_num_broad_phase_layers(_this: *const c_void) -> c_uint {
    BPL_COUNT as _
}

unsafe extern "C" fn bpl_get_broad_phase_layer(
    _this: *const c_void,
    layer: JPC_ObjectLayer,
) -> JPC_BroadPhaseLayer {
    match layer {
        OL_NON_MOVING => BPL_NON_MOVING,
        OL_MOVING => BPL_MOVING,
        _ => unreachable!(),
    }
}

const BPL: JPC_BroadPhaseLayerInterfaceFns = JPC_BroadPhaseLayerInterfaceFns {
    GetNumBroadPhaseLayers: Some(bpl_get_num_broad_phase_layers),
    GetBroadPhaseLayer: Some(bpl_get_broad_phase_layer),
};

unsafe extern "C" fn ovb_should_collide(
    _this: *const c_void,
    layer1: JPC_ObjectLayer,
    layer2: JPC_BroadPhaseLayer,
) -> bool {
    match layer1 {
        OL_NON_MOVING => layer2 == BPL_MOVING,
        OL_MOVING => true,
        _ => unreachable!(),
    }
}

const OVB: JPC_ObjectVsBroadPhaseLayerFilterFns = JPC_ObjectVsBroadPhaseLayerFilterFns {
    ShouldCollide: Some(ovb_should_collide),
};

unsafe extern "C" fn ovo_should_collide(
    _this: *const c_void,
    layer1: JPC_ObjectLayer,
    layer2: JPC_ObjectLayer,
) -> bool {
    match layer1 {
        OL_NON_MOVING => layer2 == OL_MOVING,
        OL_MOVING => true,
        _ => unreachable!(),
    }
}

const OVO: JPC_ObjectLayerPairFilterFns = JPC_ObjectLayerPairFilterFns {
    ShouldCollide: Some(ovo_should_collide),
};

/// Owns the three layer-filter interfaces required by `JPC_PhysicsSystem_Init`.
///
/// These objects must outlive the [`JPC_PhysicsSystem`] they are passed to,
/// because `Init` stores raw pointers to them. The owning [`PhysicsState`] is
/// responsible for dropping the system *before* dropping this struct.
///
/// [`PhysicsState`]: crate::physics_state::PhysicsState
pub(crate) struct LayerInterfaces {
    pub(crate) broad_phase_layer_interface: *mut JPC_BroadPhaseLayerInterface,
    pub(crate) object_vs_broad_phase_layer_filter: *mut JPC_ObjectVsBroadPhaseLayerFilter,
    pub(crate) object_layer_pair_filter: *mut JPC_ObjectLayerPairFilter,
}

impl LayerInterfaces {
    pub(crate) fn new() -> Self {
        // SAFETY: the `*Fns` vtables contain only valid `extern "C"` function
        // pointers, and we pass a null `self` pointer because our callbacks are
        // stateless.
        unsafe {
            Self {
                broad_phase_layer_interface: JPC_BroadPhaseLayerInterface_new(ptr::null(), BPL),
                object_vs_broad_phase_layer_filter: JPC_ObjectVsBroadPhaseLayerFilter_new(
                    ptr::null_mut(),
                    OVB,
                ),
                object_layer_pair_filter: JPC_ObjectLayerPairFilter_new(ptr::null_mut(), OVO),
            }
        }
    }
}

impl Drop for LayerInterfaces {
    fn drop(&mut self) {
        // SAFETY: each pointer was created by the matching `_new` in `new` and
        // is freed exactly once here. The owning `PhysicsState` guarantees the
        // physics system that referenced these has already been deleted.
        unsafe {
            JPC_BroadPhaseLayerInterface_delete(self.broad_phase_layer_interface);
            JPC_ObjectVsBroadPhaseLayerFilter_delete(self.object_vs_broad_phase_layer_filter);
            JPC_ObjectLayerPairFilter_delete(self.object_layer_pair_filter);
        }
    }
}
