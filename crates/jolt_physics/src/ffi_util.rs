//! Small helpers bridging glam/Jolt math types and Jolt shape construction.

use std::ffi::CStr;
use std::ptr;

use glam::{Quat, Vec3};
use joltc_sys::*;

/// Builds a Jolt `JPC_Vec3`. The trailing `_w` lane mirrors `z`, matching how
/// JoltC's own helpers initialise the padding lane of the SIMD vector.
pub(crate) fn vec3(x: f32, y: f32, z: f32) -> JPC_Vec3 {
    JPC_Vec3 { x, y, z, _w: z }
}

/// Builds a Jolt `JPC_RVec3` (a real-valued position vector). With
/// single-precision Jolt (our default) `Real` is `f32`; the casts only do work
/// under the `double-precision` feature where `Real` is `f64`.
#[allow(clippy::unnecessary_cast)]
pub(crate) fn rvec3(v: Vec3) -> JPC_RVec3 {
    JPC_RVec3 {
        x: v.x as Real,
        y: v.y as Real,
        z: v.z as Real,
        _w: v.z as Real,
    }
}

/// Converts a Jolt position vector back to glam, casting away precision if Jolt
/// was built with `double-precision` (a no-op cast otherwise).
#[allow(clippy::unnecessary_cast)]
pub(crate) fn rvec3_to_glam(v: JPC_RVec3) -> Vec3 {
    Vec3::new(v.x as f32, v.y as f32, v.z as f32)
}

/// Converts a Jolt quaternion to glam.
pub(crate) fn quat_to_glam(q: JPC_Quat) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

/// Creates a box shape from half-extents, panicking with Jolt's error message
/// if construction fails (e.g. degenerate extents).
pub(crate) fn create_box(half_extent: JPC_Vec3) -> *mut JPC_Shape {
    let settings = JPC_BoxShapeSettings {
        HalfExtent: half_extent,
        ..Default::default()
    };

    // SAFETY: `settings` is fully initialised and the out-pointers are valid.
    unsafe {
        let mut shape: *mut JPC_Shape = ptr::null_mut();
        let mut err: *mut JPC_String = ptr::null_mut();
        if JPC_BoxShapeSettings_Create(&settings, &mut shape, &mut err) {
            shape
        } else {
            panic!("failed to create box shape: {}", jpc_error(err));
        }
    }
}

/// Creates a sphere shape of the given radius, panicking with Jolt's error
/// message if construction fails.
pub(crate) fn create_sphere(radius: f32) -> *mut JPC_Shape {
    let settings = JPC_SphereShapeSettings {
        Radius: radius,
        ..Default::default()
    };

    // SAFETY: `settings` is fully initialised and the out-pointers are valid.
    unsafe {
        let mut shape: *mut JPC_Shape = ptr::null_mut();
        let mut err: *mut JPC_String = ptr::null_mut();
        if JPC_SphereShapeSettings_Create(&settings, &mut shape, &mut err) {
            shape
        } else {
            panic!("failed to create sphere shape: {}", jpc_error(err));
        }
    }
}

/// Reads a `JPC_String` error into an owned Rust string.
///
/// # Safety
/// `err` must be a valid pointer returned by a JoltC `*_Create` call.
unsafe fn jpc_error(err: *mut JPC_String) -> String {
    if err.is_null() {
        return "unknown error".to_string();
    }
    CStr::from_ptr(JPC_String_c_str(err))
        .to_string_lossy()
        .into_owned()
}
