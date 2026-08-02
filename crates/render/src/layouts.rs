use std::{num::NonZero, ops::Deref};

use derive_more::Deref;
use ecs::resource::Resource;
use wgpu::BindGroupLayoutDescriptor;

use crate::components::light::MAX_SHADOW_CASTERS;

/// Bind-group layout for the camera uniform (`@group(1) @binding(0)` in the
/// default material convention).
///
/// Exposed publicly so crates with their own render passes (e.g. debug gizmos)
/// can build a pipeline whose camera bind-group layout is *the same object*
/// that [`RenderCamera`](crate::components::camera::RenderCamera) bind groups
/// are created from, guaranteeing wgpu bind-group compatibility.
#[derive(Resource)]
pub struct CameraLayout {
    pub camera_layout: wgpu::BindGroupLayout,
}

impl CameraLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        Self {
            camera_layout: camera_bind_group_layout,
        }
    }
}

#[derive(Resource)]
pub(crate) struct LightLayout(wgpu::BindGroupLayout);

impl LightLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let lights_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lights_bind_group_layout"),
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
        });

        Self(lights_layout)
    }
}

impl Deref for LightLayout {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Resource, Deref)]
pub(crate) struct SkeletonLayout(pub(crate) wgpu::BindGroupLayout);

impl SkeletonLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let skeleton_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("skeleton_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        Self(skeleton_layout)
    }
}

#[derive(Resource, Deref)]
pub(crate) struct ShadowsLayout(pub(crate) wgpu::BindGroupLayout);

impl ShadowsLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let skeleton_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("shadows_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        Self(skeleton_layout)
    }
}
