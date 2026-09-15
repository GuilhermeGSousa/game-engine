use glam::{EulerRot, Quat, Vec3};

use crate::{Editable, EditorValue};

fn to_vec3(value: [f64; 3]) -> Vec3 {
    Vec3::new(value[0] as f32, value[1] as f32, value[2] as f32)
}

fn from_vec3(value: Vec3) -> EditorValue {
    EditorValue::Vec3([value.x as f64, value.y as f64, value.z as f64])
}

impl Editable for f32 {
    fn read(&self) -> Option<EditorValue> {
        Some(EditorValue::Number(*self as f64))
    }

    fn write(&mut self, value: &EditorValue) -> bool {
        let EditorValue::Number(number) = value else {
            return false;
        };
        // Checked after narrowing: a finite f64 can overflow f32.
        let number = *number as f32;
        if !number.is_finite() {
            return false;
        }
        *self = number;
        true
    }
}

impl Editable for f64 {
    fn read(&self) -> Option<EditorValue> {
        Some(EditorValue::Number(*self))
    }

    fn write(&mut self, value: &EditorValue) -> bool {
        let EditorValue::Number(number) = value else {
            return false;
        };
        if !number.is_finite() {
            return false;
        }
        *self = *number;
        true
    }
}

impl Editable for Vec3 {
    fn read(&self) -> Option<EditorValue> {
        Some(from_vec3(*self))
    }

    fn write(&mut self, value: &EditorValue) -> bool {
        let EditorValue::Vec3(components) = value else {
            return false;
        };
        let vector = to_vec3(*components);
        if !vector.is_finite() {
            return false;
        }
        *self = vector;
        true
    }
}

impl Editable for Quat {
    fn read(&self) -> Option<EditorValue> {
        let (x, y, z) = self.to_euler(EulerRot::XYZ);
        Some(from_vec3(Vec3::new(x, y, z).map(f32::to_degrees)))
    }

    fn write(&mut self, value: &EditorValue) -> bool {
        let EditorValue::Vec3(degrees) = value else {
            return false;
        };
        let radians = to_vec3(*degrees).map(f32::to_radians);
        if !radians.is_finite() {
            return false;
        }
        *self = Quat::from_euler(EulerRot::XYZ, radians.x, radians.y, radians.z).normalize();
        true
    }
}

#[cfg(test)]
mod tests {
    use glam::{EulerRot, Quat, Vec3};

    use crate::{Editable, EditorValue};

    #[test]
    fn scalars_round_trip() {
        let mut value = 0.0_f32;
        assert!(value.write(&EditorValue::Number(2.5)));
        assert_eq!(value.read(), Some(EditorValue::Number(2.5)));

        let mut value = 0.0_f64;
        assert!(value.write(&EditorValue::Number(-7.25)));
        assert_eq!(value.read(), Some(EditorValue::Number(-7.25)));
    }

    #[test]
    fn vec3_round_trips() {
        let mut value = Vec3::ZERO;
        assert!(value.write(&EditorValue::Vec3([1.0, 2.0, 3.0])));
        assert_eq!(value, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(value.read(), Some(EditorValue::Vec3([1.0, 2.0, 3.0])));
    }

    #[test]
    fn quat_is_presented_as_euler_degrees() {
        let quarter_turn = Quat::from_rotation_y(90_f32.to_radians());
        let Some(EditorValue::Vec3([x, y, z])) = quarter_turn.read() else {
            panic!("a Quat must read as a Vec3 of degrees");
        };
        assert!(x.abs() < 1e-3 && (y - 90.0).abs() < 1e-3 && z.abs() < 1e-3);
    }

    #[test]
    fn quat_round_trips_through_degrees() {
        let original = Quat::from_euler(EulerRot::XYZ, 0.3, -1.1, 2.0);
        let mut copy = Quat::IDENTITY;
        assert!(copy.write(&original.read().unwrap()));
        assert!(
            original.dot(copy).abs() > 1.0 - 1e-5,
            "q and -q are the same rotation; compare with |dot|"
        );
    }

    #[test]
    fn non_finite_values_are_rejected_and_leave_the_leaf_unchanged() {
        let mut scalar = 1.0_f32;
        assert!(!scalar.write(&EditorValue::Number(f64::NAN)));
        assert!(!scalar.write(&EditorValue::Number(1e300)), "overflows f32");
        assert_eq!(scalar, 1.0);

        let mut vector = Vec3::ONE;
        assert!(!vector.write(&EditorValue::Vec3([0.0, f64::INFINITY, 0.0])));
        assert_eq!(vector, Vec3::ONE);

        let mut rotation = Quat::IDENTITY;
        assert!(!rotation.write(&EditorValue::Vec3([f64::NAN, 0.0, 0.0])));
        assert_eq!(rotation, Quat::IDENTITY);
    }

    #[test]
    fn wrong_shapes_are_rejected() {
        let mut scalar = 1.0_f32;
        assert!(!scalar.write(&EditorValue::Vec3([0.0; 3])));
        let mut vector = Vec3::ONE;
        assert!(!vector.write(&EditorValue::Number(0.0)));
        assert_eq!(vector, Vec3::ONE);
    }
}
