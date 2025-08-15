use std::io::{BufReader, Cursor};

use async_trait::async_trait;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, utils::load_to_string, Asset,
    AssetPath,
};

use crate::assets::{material::Material, texture::Texture};

pub(crate) struct MTLLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for MTLLoader {
    type Asset = Material;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        usage_setting: <Self::Asset as Asset>::UsageSettings,
    ) -> Result<Self::Asset, ()> {
        let obj_text = load_to_string(path.clone()).await?;
        let obj_cursor = Cursor::new(obj_text);
        let mat = tobj::load_mtl_buf(&mut BufReader::new(obj_cursor));

        match mat {
            Ok((mats, _)) => {
                let mut material = Material::new();
                for m in mats {
                    if let Some(diffuse_texture) = m.diffuse_texture {
                        let texture_handle =
                            load_context.asset_server().load::<Texture>(diffuse_texture);
                        material.set_diffuse_texture(texture_handle);
                    }

                    if let Some(normal_texture) = m.normal_texture {
                        let texture_handle =
                            load_context.asset_server().load::<Texture>(normal_texture);
                        material.set_normal_texture(texture_handle);
                    }
                }
                return Ok(material);
            }
            Err(_) => {
                eprintln!("Failed to load MTL file: {}", path.to_path().display());
                return Err(());
            }
        }
    }
}
