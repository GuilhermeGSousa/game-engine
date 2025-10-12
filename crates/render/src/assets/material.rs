use bytemuck::{Pod, Zeroable};
use essential::assets::{handle::AssetHandle, Asset, LoadableAsset};

use super::texture::Texture;
use crate::loaders::mtl_loader::MTLLoader;
use bitflags::bitflags;

pub struct Material {
    diffuse_texture: Option<AssetHandle<Texture>>,
    normal_texture: Option<AssetHandle<Texture>>,
}

impl Material {
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

impl Asset for Material {}

impl LoadableAsset for Material {
    type UsageSettings = ();
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(MTLLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
}

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
    pub(crate) fn from_material(material: &Material) -> Self {
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
