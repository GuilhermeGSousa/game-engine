use crate::loaders::texture_loader::TextureLoader;
use essential::assets::{Asset, LoadableAsset};
use image::{DynamicImage, GenericImageView};
use wgpu::TextureUsages;
use wgpu_types::{Extent3d, TextureDescriptor, TextureFormat, TextureViewDescriptor};

pub struct TextureUsageSettings {
    pub texture_descriptor: TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>,
    pub texture_view_descriptor: TextureViewDescriptor<Option<&'static str>>,
}

impl Default for TextureUsageSettings {
    fn default() -> Self {
        Self {
            texture_descriptor: TextureDescriptor {
                label: Some("texture"),
                size: Extent3d {
                    width: 0,
                    height: 0,
                    depth_or_array_layers: 0,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            texture_view_descriptor: TextureViewDescriptor {
                label: Some("texture_view"),
                ..Default::default()
            },
        }
    }
}

pub struct Texture {
    data: Vec<u8>,
    usage_settings: TextureUsageSettings,
}

impl Texture {
    pub fn from_bytes(bytes: &[u8], mut usage_settings: TextureUsageSettings) -> Result<Self, ()> {
        let img = image::load_from_memory(bytes).map_err(|_| ())?;
        let dimensions = img.dimensions();

        if usage_settings.texture_descriptor.size.width == 0
            && usage_settings.texture_descriptor.size.height == 0
            && usage_settings.texture_descriptor.size.depth_or_array_layers == 0
        {
            usage_settings.texture_descriptor.size = Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };
        }

        Ok(Texture {
            data: img.to_rgba8().into_raw(),
            usage_settings,
        })
    }

    pub fn from_dynamic_image(image: DynamicImage) -> Self {
        let mut usage_settings = TextureUsageSettings::default();

        let extent = Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };

        usage_settings.texture_descriptor.size = extent;
        usage_settings.texture_descriptor.format = TextureFormat::Rgba8UnormSrgb;

        Self {
            data: image.to_rgba8().into_raw(),
            usage_settings: usage_settings,
        }
    }

    pub fn size(&self) -> &wgpu::Extent3d {
        &self.usage_settings.texture_descriptor.size
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn usage_settings(&self) -> &TextureUsageSettings {
        &self.usage_settings
    }
}

impl Asset for Texture {}

impl LoadableAsset for Texture {
    type UsageSettings = TextureUsageSettings;

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(TextureLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        TextureUsageSettings::default()
    }
}
