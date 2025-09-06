use async_trait::async_trait;
use essential::assets::asset_loader::AssetLoader;
use gltf::{Document, Gltf};

use crate::assets::scene::Scene;
pub(crate) struct GLTFLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for GLTFLoader {
    type Asset = Scene;

    async fn load(
        &self,
        path: essential::assets::AssetPath<'static>,
        load_context: &mut essential::assets::asset_server::AssetLoadContext,
        usage_setting: <Self::Asset as essential::assets::Asset>::UsageSettings,
    ) -> Result<Self::Asset, ()> {
        let (document, buffers, images) = gltf::import("foo").unwrap();

        let meshes = document.meshes().map(|mesh| {
            println!("here");

            mesh.primitives().map(|primitive| {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                if let Some(indices) = reader.read_indices() {}

                if let Some(positions) = reader.read_positions() {}

                if let Some(normals) = reader.read_normals() {}

                if let Some(tangents) = reader.read_tangents() {}

                if let Some(tex_coords_0) = reader.read_tex_coords(0) {}

                if let Some(joints_0) = reader.read_joints(0) {}

                if let Some(weights_0) = reader.read_weights(0) {}
            })
        });
        todo!()
    }
}
