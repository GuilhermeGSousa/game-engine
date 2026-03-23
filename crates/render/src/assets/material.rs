use bytemuck::{Pod, Zeroable};
use essential::assets::{handle::AssetHandle, Asset};

use crate::render_asset::{
    render_texture::{DummyRenderTexture, RenderTexture},
    AssetPreparationError, RenderAssets,
};

use super::texture::Texture;

use bitflags::bitflags;

// ────────────────────────────────────────────────────────────────────────────
// ShaderRef — reference to a WGSL shader used by a material
// ────────────────────────────────────────────────────────────────────────────

/// A reference to a WGSL shader source.
///
/// Pass [`ShaderRef::Default`] to use the engine's built-in Phong shader, or
/// [`ShaderRef::Source`] to supply your own WGSL source string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShaderRef {
    /// Use the engine's built-in shader.
    Default,
    /// Use the provided WGSL source string.
    Source(&'static str),
}

// ────────────────────────────────────────────────────────────────────────────
// AsBindGroup — trait implemented (manually or via derive) by material types
// ────────────────────────────────────────────────────────────────────────────

/// Describes a type that can produce a wgpu bind-group layout and a bind-group
/// populated from its own data.
///
/// Implement this trait manually, or use `#[derive(AsBindGroup)]` from the
/// `render-macros` crate to generate the implementation automatically.
///
/// # Example (manual implementation)
///
/// ```rust,ignore
/// use render::assets::material::{AsBindGroup, ShaderRef};
///
/// pub struct MyMaterial {
///     pub tint: [f32; 4],
/// }
///
/// impl AsBindGroup for MyMaterial {
///     fn vertex_shader() -> ShaderRef { ShaderRef::Default }
///     fn fragment_shader() -> ShaderRef { ShaderRef::Source(include_str!("my.wgsl")) }
///
///     fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout { … }
///     fn create_bind_group(&self, …) -> Result<wgpu::BindGroup, …> { … }
/// }
/// ```
pub trait AsBindGroup {
    /// The vertex shader to use when rendering with this material.
    fn vertex_shader() -> ShaderRef
    where
        Self: Sized,
    {
        ShaderRef::Default
    }

    /// The fragment shader to use when rendering with this material.
    fn fragment_shader() -> ShaderRef
    where
        Self: Sized,
    {
        ShaderRef::Default
    }

    /// Build the `BindGroupLayout` that describes the shader bindings for this material.
    fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout
    where
        Self: Sized;

    /// Create a `BindGroup` populated with the material's actual GPU resources.
    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        render_textures: &RenderAssets<RenderTexture>,
        dummy_texture: &DummyRenderTexture,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<wgpu::BindGroup, AssetPreparationError>;
}

// ────────────────────────────────────────────────────────────────────────────
// StandardMaterial — the engine's built-in Phong material
// ────────────────────────────────────────────────────────────────────────────

/// The engine's built-in Phong-shaded material with optional diffuse and normal
/// maps.
///
/// Use this as a starting point, or define your own material by implementing
/// [`AsBindGroup`] (manually or via `#[derive(AsBindGroup)]`).
#[derive(Asset)]
pub struct StandardMaterial {
    diffuse_texture: Option<AssetHandle<Texture>>,
    normal_texture: Option<AssetHandle<Texture>>,
}

impl StandardMaterial {
    pub fn new(
        diffuse_texture: Option<AssetHandle<Texture>>,
        normal_texture: Option<AssetHandle<Texture>>,
    ) -> Self {
        Self {
            diffuse_texture,
            normal_texture,
        }
    }

    pub fn set_diffuse_texture(&mut self, texture: AssetHandle<Texture>) {
        self.diffuse_texture = Some(texture);
    }

    pub fn diffuse_texture(&self) -> Option<&AssetHandle<Texture>> {
        self.diffuse_texture.as_ref()
    }

    pub fn set_normal_texture(&mut self, texture: AssetHandle<Texture>) {
        self.normal_texture = Some(texture);
    }

    pub fn normal_texture(&self) -> Option<&AssetHandle<Texture>> {
        self.normal_texture.as_ref()
    }
}

impl AsBindGroup for StandardMaterial {
    fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("StandardMaterial_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        render_textures: &RenderAssets<RenderTexture>,
        dummy_texture: &DummyRenderTexture,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<wgpu::BindGroup, AssetPreparationError> {
        use wgpu::util::DeviceExt;
        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = Vec::new();

        if let Some(diffuse_tex_handle) = self.diffuse_texture() {
            if let Some(diffuse_tex) = render_textures.get(&diffuse_tex_handle.id()) {
                entries.push(wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_tex.view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_tex.sampler),
                });
            } else {
                return Err(AssetPreparationError::NotReady);
            }
        } else {
            entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dummy_texture.view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
            });
        }

        if let Some(normal_tex_handle) = self.normal_texture() {
            if let Some(normal_tex) = render_textures.get(&normal_tex_handle.id()) {
                entries.push(wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_tex.view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_tex.sampler),
                });
            } else {
                return Err(AssetPreparationError::NotReady);
            }
        } else {
            entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&dummy_texture.view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
            });
        }

        let flags = MaterialFlags::from_material(self);
        let material_flags_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("StandardMaterial_flags"),
                contents: bytemuck::cast_slice(&[MaterialUniform {
                    flags,
                    _padding: [0; 3],
                    _padding2: [0; 4],
                }]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Buffer(
                material_flags_buffer.as_entire_buffer_binding(),
            ),
        });

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &entries,
            label: Some("StandardMaterial_bind_group"),
        }))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// MaterialFlags / MaterialUniform (used by StandardMaterial)
// ────────────────────────────────────────────────────────────────────────────

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MaterialFlags(u32);

bitflags! {
    impl MaterialFlags: u32 {
        const HAS_DIFFUSE_TEXTURE = 1 << 0;
        const HAS_NORMAL_TEXTURE = 1 << 1;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct MaterialUniform {
    pub(crate) flags: MaterialFlags,
    pub(crate) _padding: [u32; 3],
    pub(crate) _padding2: [u32; 4],
}

impl MaterialFlags {
    pub(crate) fn from_material(material: &StandardMaterial) -> Self {
        let mut flags: MaterialFlags = MaterialFlags(0);
        if material.diffuse_texture.is_some() {
            flags |= MaterialFlags::HAS_DIFFUSE_TEXTURE;
        }
        if material.normal_texture.is_some() {
            flags |= MaterialFlags::HAS_NORMAL_TEXTURE;
        }
        flags
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Backward-compatibility alias
// ────────────────────────────────────────────────────────────────────────────

/// Backward-compatible alias for [`StandardMaterial`].
///
/// New code should prefer `StandardMaterial` directly.
#[deprecated(since = "0.1.0", note = "Use `StandardMaterial` instead")]
pub type Material = StandardMaterial;
