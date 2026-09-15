extern crate self as editable;

mod leaves;
mod property;

use std::any::Any;

/// Implements [`Editable`] for a struct with named fields by visiting each
/// field in declaration order. Every field must itself be `Editable`.
///
/// ```compile_fail
/// #[derive(editable::Editable)]
/// struct Tuple(f32);
/// ```
///
/// ```compile_fail
/// #[derive(editable::Editable)]
/// enum Choice { A, B }
/// ```
///
/// ```compile_fail
/// struct Opaque;
///
/// #[derive(editable::Editable)]
/// struct HasOpaque { inner: Opaque }
/// ```
pub use editable_macros::Editable;
pub use property::{ApplyError, Property, PropertyPath, apply, collect};

/// The value of one leaf in the form the editor shows and edits, which is not
/// necessarily its Rust form: a `Quat` is presented as euler degrees.
#[derive(Clone, Debug, PartialEq)]
pub enum EditorValue {
    Number(f64),
    Vec3([f64; 3]),
}

/// A type the editor can show and change. A leaf answers `read`/`write`; a
/// struct answers `visit`/`visit_mut`, usually through `#[derive(Editable)]`.
pub trait Editable: Any {
    /// `Some` for a leaf, `None` for a struct.
    fn read(&self) -> Option<EditorValue> {
        None
    }

    /// Returns `false` when `value` has the wrong shape or is not finite; the
    /// leaf is then left unchanged.
    fn write(&mut self, _value: &EditorValue) -> bool {
        false
    }

    fn visit(&self, _visitor: &mut dyn PropertyVisitor) {}

    fn visit_mut(&mut self, _visitor: &mut dyn PropertyVisitorMut) {}
}

pub trait PropertyVisitor {
    fn field(&mut self, name: &'static str, value: &dyn Editable);
}

pub trait PropertyVisitorMut {
    fn field(&mut self, name: &'static str, value: &mut dyn Editable);
}
