use ecs::component::Component;

#[derive(Component)]
pub struct UIMaterialComponent {
    pub color: wgpu::Color,
}
