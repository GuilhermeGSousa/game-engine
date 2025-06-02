use std::io::{BufReader, Cursor};

use async_trait::async_trait;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, utils::load_to_string, AssetPath,
};

use crate::assets::{material::Material, texture::Texture};

pub(crate) struct MTLLoader;

#[async_trait]
impl AssetLoader for MTLLoader {
    type Asset = Material;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
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
