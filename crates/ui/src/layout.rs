use ecs::resource::Resource;

#[derive(Resource)]
pub(crate) struct UICameraLayout(wgpu::BindGroupLayout);