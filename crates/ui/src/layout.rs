use derive_more::Deref;
use ecs::resource::Resource;
use wgpu::BindGroupLayoutDescriptor;

#[derive(Resource, Deref)]
pub(crate) struct UICameraLayout(wgpu::BindGroupLayout);

impl UICameraLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("ui_camera_bind_group_layout"),
            });

        Self(camera_bind_group_layout)
    }
}

#[derive(Resource, Deref)]
pub(crate) struct UIMaterialLayout(wgpu::BindGroupLayout);

impl UIMaterialLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("ui_material_bind_group_layout"),
        }))
    }
}
