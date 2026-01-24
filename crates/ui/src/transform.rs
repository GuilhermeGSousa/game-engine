use ecs::component::Component;

pub enum UIValue {
    Px(f32),
    Percernt(f32),
}

pub struct UIValue2 {
    pub x: UIValue,
    pub y: UIValue,
}

#[derive(Component)]
pub struct UITransform {
    pub translation: UIValue2,
}

pub struct GlobalUITransformRaw;
