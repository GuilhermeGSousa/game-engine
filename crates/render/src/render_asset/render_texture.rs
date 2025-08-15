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
pub(crate) struct RenderTexture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
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

        Self {
            texture: wgpu_texture,
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
pub(crate) struct DummyRenderTexture(pub(crate) RenderTexture);

impl DummyRenderTexture {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
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
}

impl Deref for DummyRenderTexture {
    type Target = RenderTexture;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
