use std::any::TypeId;

use editable::{Editable, EditorValue, Property, PropertyPath, apply, collect};
use glam::{Quat, Vec3};

#[derive(Editable)]
struct Inner {
    weight: f32,
}

#[derive(Editable)]
struct Outer {
    position: Vec3,
    rotation: Quat,
    inner: Inner,
}

fn outer() -> Outer {
    Outer {
        position: Vec3::X,
        rotation: Quat::IDENTITY,
        inner: Inner { weight: 1.0 },
    }
}

#[test]
fn derived_structs_collect_nested_leaves_in_declaration_order() {
    let properties = collect(&outer());
    let shape: Vec<_> = properties
        .iter()
        .map(|property| (property.path.segments().to_vec(), property.type_id))
        .collect();
    assert_eq!(
        shape,
        vec![
            (vec!["position"], TypeId::of::<Vec3>()),
            (vec!["rotation"], TypeId::of::<Quat>()),
            (vec!["inner", "weight"], TypeId::of::<f32>()),
        ]
    );
    assert_eq!(
        properties[2],
        Property {
            path: PropertyPath::new(["inner", "weight"]),
            type_id: TypeId::of::<f32>(),
            value: EditorValue::Number(1.0),
        }
    );
}

#[test]
fn derived_structs_apply_through_visit_mut() {
    let mut value = outer();
    apply(
        &mut value,
        &PropertyPath::new(["inner", "weight"]),
        &EditorValue::Number(4.0),
    )
    .unwrap();
    assert_eq!(value.inner.weight, 4.0);
}

#[test]
fn a_derived_struct_is_not_a_leaf() {
    assert_eq!(outer().read(), None);
}
