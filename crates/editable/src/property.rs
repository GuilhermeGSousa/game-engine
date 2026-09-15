use std::any::{Any, TypeId};

use crate::{Editable, EditorValue, PropertyVisitor, PropertyVisitorMut};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PropertyPath(Vec<&'static str>);

impl PropertyPath {
    pub fn new(segments: impl IntoIterator<Item = &'static str>) -> Self {
        Self(segments.into_iter().collect())
    }

    pub fn segments(&self) -> &[&'static str] {
        &self.0
    }

    pub fn name(&self) -> &'static str {
        self.0.last().copied().unwrap_or("")
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn get_depth(&self, index: usize) -> Option<&'static str> {
        self.0.get(index).map(|v| &**v)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub path: PropertyPath,
    pub type_id: TypeId,
    pub value: EditorValue,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApplyError {
    NotFound,
    Rejected,
}

pub fn collect(root: &dyn Editable) -> Vec<Property> {
    let mut collector = Collect {
        path: Vec::new(),
        properties: Vec::new(),
    };
    root.visit(&mut collector);
    collector.properties
}

pub fn apply(
    root: &mut dyn Editable,
    path: &PropertyPath,
    value: &EditorValue,
) -> Result<(), ApplyError> {
    let mut applier = Apply {
        target: path,
        depth: 0,
        value,
        result: None,
    };
    root.visit_mut(&mut applier);
    applier.result.unwrap_or(Err(ApplyError::NotFound))
}

struct Collect {
    path: Vec<&'static str>,
    properties: Vec<Property>,
}

impl PropertyVisitor for Collect {
    fn field(&mut self, name: &'static str, value: &dyn Editable) {
        self.path.push(name);
        match value.read() {
            Some(leaf) => self.properties.push(Property {
                path: PropertyPath(self.path.clone()),
                // Must upcast: `.type_id()` on a `&&dyn Editable` is the reference's own id.
                type_id: (value as &dyn Any).type_id(),
                value: leaf,
            }),
            None => value.visit(self),
        }
        self.path.pop();
    }
}

struct Apply<'a> {
    target: &'a PropertyPath,
    depth: usize,
    value: &'a EditorValue,
    result: Option<Result<(), ApplyError>>,
}

impl PropertyVisitorMut for Apply<'_> {
    fn field(&mut self, name: &'static str, value: &mut dyn Editable) {
        if self.result.is_some() || self.target.get_depth(self.depth) != Some(&name) {
            return;
        }
        let at_end = self.depth + 1 == self.target.depth();
        match (at_end, value.read().is_some()) {
            (true, true) => {
                self.result = Some(if value.write(self.value) {
                    Ok(())
                } else {
                    Err(ApplyError::Rejected)
                });
            }
            (false, false) => {
                self.depth += 1;
                value.visit_mut(self);
                self.depth -= 1;
            }
            _ => self.result = Some(Err(ApplyError::NotFound)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use glam::Vec3;

    use super::*;
    use crate::{PropertyVisitor, PropertyVisitorMut};

    struct Inner {
        weight: f32,
    }

    impl Editable for Inner {
        fn visit(&self, visitor: &mut dyn PropertyVisitor) {
            visitor.field("weight", &self.weight);
        }
        fn visit_mut(&mut self, visitor: &mut dyn PropertyVisitorMut) {
            visitor.field("weight", &mut self.weight);
        }
    }

    struct Outer {
        position: Vec3,
        inner: Inner,
    }

    impl Editable for Outer {
        fn visit(&self, visitor: &mut dyn PropertyVisitor) {
            visitor.field("position", &self.position);
            visitor.field("inner", &self.inner);
        }
        fn visit_mut(&mut self, visitor: &mut dyn PropertyVisitorMut) {
            visitor.field("position", &mut self.position);
            visitor.field("inner", &mut self.inner);
        }
    }

    fn outer() -> Outer {
        Outer {
            position: Vec3::new(1.0, 2.0, 3.0),
            inner: Inner { weight: 0.5 },
        }
    }

    #[test]
    fn collect_flattens_nested_leaves_in_field_order() {
        assert_eq!(
            collect(&outer()),
            vec![
                Property {
                    path: PropertyPath::new(["position"]),
                    type_id: TypeId::of::<Vec3>(),
                    value: EditorValue::Vec3([1.0, 2.0, 3.0]),
                },
                Property {
                    path: PropertyPath::new(["inner", "weight"]),
                    type_id: TypeId::of::<f32>(),
                    value: EditorValue::Number(0.5),
                },
            ]
        );
    }

    #[test]
    fn apply_writes_exactly_one_nested_leaf() {
        let mut value = outer();
        let result = apply(
            &mut value,
            &PropertyPath::new(["inner", "weight"]),
            &EditorValue::Number(2.0),
        );
        assert_eq!(result, Ok(()));
        assert_eq!(value.inner.weight, 2.0);
        assert_eq!(value.position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn a_path_ending_on_a_struct_is_not_found() {
        let result = apply(
            &mut outer(),
            &PropertyPath::new(["inner"]),
            &EditorValue::Number(2.0),
        );
        assert_eq!(result, Err(ApplyError::NotFound));
    }

    #[test]
    fn a_path_continuing_past_a_leaf_is_not_found() {
        let result = apply(
            &mut outer(),
            &PropertyPath::new(["position", "x"]),
            &EditorValue::Number(2.0),
        );
        assert_eq!(result, Err(ApplyError::NotFound));
    }

    #[test]
    fn an_unknown_or_empty_path_is_not_found() {
        let value = EditorValue::Number(2.0);
        assert_eq!(
            apply(&mut outer(), &PropertyPath::new(["missing"]), &value),
            Err(ApplyError::NotFound)
        );
        assert_eq!(
            apply(&mut outer(), &PropertyPath::default(), &value),
            Err(ApplyError::NotFound)
        );
    }

    #[test]
    fn a_rejected_value_leaves_the_leaf_unchanged() {
        let mut value = outer();
        let result = apply(
            &mut value,
            &PropertyPath::new(["position"]),
            &EditorValue::Number(2.0),
        );
        assert_eq!(result, Err(ApplyError::Rejected));
        assert_eq!(value.position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn path_name_is_the_last_segment() {
        assert_eq!(PropertyPath::new(["inner", "weight"]).name(), "weight");
        assert_eq!(PropertyPath::default().name(), "");
    }
}
