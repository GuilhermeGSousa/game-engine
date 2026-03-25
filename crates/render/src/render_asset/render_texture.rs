use std::ops::Deref;

use crate::{
    assets::texture::Texture,
    device::RenderDevice,
    queue::RenderQueue,
    render_asset::{AssetPreparationError, RenderAsset},
};
use ecs::{
    resource::{Res, Resource},
    system::system_input::SystemInputData,
};

#[allow(dead_code)]
pub struct RenderTexture {
    /// The GPU texture view (used for bind groups).
    pub view: wgpu::TextureView,
    /// The GPU sampler associated with this texture.
    pub sampler: wgpu::Sampler,
    /// The underlying GPU texture.  Kept crate-private since callers interact
    /// with `view` and `sampler`; expose via a getter if broader access is needed.
    pub(crate) texture: wgpu::Texture,
}

impl RenderTexture {
    pub fn from_texture(texture: &Texture, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RenderTexture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let dimensions = texture.size();
        let usage_settings = texture.usage_settings();
        let wgpu_texture = device.create_texture(&usage_settings.texture_descriptor);
        let view = wgpu_texture.create_view(&usage_settings.texture_view_descriptor);

        // Render-target textures (created with `Texture::new_render_target`) have no
        // initial pixel data; skip the upload in that case.
        if !texture.data().is_empty() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::All,
                    texture: &wgpu_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &texture.data(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * dimensions.width),
                    rows_per_image: Some(dimensions.height),
                },
                *dimensions,
            );
        }

        Self {
            texture: wgpu_texture,
            view,
            sampler,
        }
    }

    /// Creates a depth texture sized to match a render target of the given pixel dimensions.
    pub fn create_depth_texture_sized(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
        }
    }

    /// Creates a colour texture suitable for use as a render target.
    ///
    /// The resulting [`RenderTexture`] has `RENDER_ATTACHMENT | TEXTURE_BINDING` usage
    /// so it can be both rendered into by a camera and sampled as a texture (e.g. in
    /// a UI viewport element).
    ///
    /// `format` should match the surface/pipeline format so that the existing render
    /// pipelines can write to it without reconfiguration.
    ///
    /// Note: prefer using [`Texture::new_render_target`] together with the asset system
    /// rather than calling this directly; this method is kept for convenience.
    pub fn create_color_render_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Color Render Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Color Render Target Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            // 2.
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // 4.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

impl RenderAsset for RenderTexture {
    type SourceAsset = Texture;

    type PreparationParams = (Res<'static, RenderDevice>, Res<'static, RenderQueue>);

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError> {
        let (device, queue) = params;
        Ok(RenderTexture::from_texture(&source_asset, &device, &queue))
    }
}

#[derive(Resource)]
pub struct DummyRenderTexture(pub(crate) RenderTexture);

impl DummyRenderTexture {
    pub fn new(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RenderTexture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self(RenderTexture {
            texture: wgpu_texture,
            view: view,
            sampler: sampler,
        })
    }

    /// Access the inner [`RenderTexture`].
    ///
    /// In most cases the `Deref` impl is sufficient (use `dummy.view` or
    /// `dummy.sampler`); use this getter when you need the full `RenderTexture`.
    pub fn inner(&self) -> &RenderTexture {
        &self.0
    }
}

impl Deref for DummyRenderTexture {
    type Target = RenderTexture;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
