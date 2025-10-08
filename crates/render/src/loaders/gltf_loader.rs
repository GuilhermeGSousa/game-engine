use async_trait::async_trait;
use essential::assets::{asset_loader::AssetLoader, Asset, LoadableAsset};

use crate::assets::{texture::{Texture, TextureUsageSettings}, vertex::Vertex};
pub(crate) struct GLTFLoader;

pub struct GLTFScene
{

}

impl Asset for GLTFScene {
    
}

impl LoadableAsset for GLTFScene {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(GLTFLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for GLTFLoader {
    type Asset = GLTFScene;

    async fn load(
        &self,
        path: essential::assets::AssetPath<'static>,
        load_context: &mut essential::assets::asset_server::AssetLoadContext,
        usage_setting: <Self::Asset as essential::assets::LoadableAsset>::UsageSettings,
    ) -> Result<Self::Asset, ()> {
        let (document, buffers, images) = gltf::import(path.to_path()).unwrap();

        let mut textures = images.iter().enumerate().map(|(index, image)| {
            if let Ok(asset) = Texture::from_bytes(&image.pixels, TextureUsageSettings::default()) {
                Some(load_context.asset_server().add(asset))
            } else {
                None
            }
        });

        // if textures.any(|handle| handle.is_none()) {
        //     return Err(());
        // }

        // let textures: Vec<_> = textures.map(|handle| handle.unwrap()).collect();

        // let meshes = document
        //     .meshes()
        //     .map(|mesh: gltf::Mesh<'_>| {
        //         let mut primitives = mesh.primitives().map(|gltf_primitive| {
        //             let mut primitive = Primitive {
        //                 vertices: Vec::new(),
        //                 indices: Vec::new(),
        //                 material_index: 0,
        //             };

        //             gltf_primitive
        //                 .material()
        //                 .occlusion_texture()
        //                 .unwrap()
        //                 .texture()
        //                 .index();

        //             let reader = gltf_primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        //             primitive.indices = match reader.read_indices()? {
        //                 gltf::mesh::util::ReadIndices::U8(iter) => iter.map(|i| i as u32).collect(),
        //                 gltf::mesh::util::ReadIndices::U16(iter) => {
        //                     iter.map(|i| i as u32).collect()
        //                 }
        //                 gltf::mesh::util::ReadIndices::U32(iter) => iter.collect(),
        //             };

        //             primitive.vertices = reader
        //                 .read_positions()?
        //                 .map(|pos| Vertex {
        //                     pos_coords: pos,
        //                     uv_coords: [0.0; 2],
        //                     normal: [0.0, 0.0, 1.0],
        //                     tangent: [0.0; 3],
        //                     bitangent: [0.0; 3],
        //                     bone_indices: [0; Vertex::MAX_AFFECTED_BONES],
        //                     bone_weights: [0.0; Vertex::MAX_AFFECTED_BONES],
        //                 })
        //                 .collect();

        //             if let Some(uv_0) = reader.read_tex_coords(0) {
        //                 match uv_0 {
        //                     gltf::mesh::util::ReadTexCoords::F32(iter) => {
        //                         iter.enumerate().for_each(|(index, uvs)| {
        //                             primitive.vertices[index].uv_coords = uvs;
        //                         });
        //                     }
        //                     _ => return None,
        //                 }
        //             }

        //             if let Some(normals) = reader.read_normals() {
        //                 normals.enumerate().for_each(|(index, normal)| {
        //                     primitive.vertices[index].normal = normal;
        //                 });
        //             }

        //             if let Some(tangents) = reader.read_tangents() {
        //                 tangents.enumerate().for_each(|(index, tangent)| {
        //                     primitive.vertices[index].tangent =
        //                         [tangent[0], tangent[1], tangent[2]];
        //                 });
        //             }

        //             if let Some(joints_0) = reader.read_joints(0) {
        //                 match joints_0 {
        //                     gltf::mesh::util::ReadJoints::U8(iter) => {
        //                         iter.enumerate().for_each(|(index, joint)| {
        //                             primitive.vertices[index].bone_indices = [
        //                                 joint[0].into(),
        //                                 joint[1].into(),
        //                                 joint[2].into(),
        //                                 joint[3].into(),
        //                             ];
        //                         })
        //                     }
        //                     gltf::mesh::util::ReadJoints::U16(iter) => {
        //                         iter.enumerate().for_each(|(index, joint)| {
        //                             primitive.vertices[index].bone_indices = [
        //                                 joint[0].into(),
        //                                 joint[1].into(),
        //                                 joint[2].into(),
        //                                 joint[3].into(),
        //                             ];
        //                         })
        //                     }
        //                 }
        //             }

        //             if let Some(weights_0) = reader.read_weights(0) {
        //                 match weights_0 {
        //                     gltf::mesh::util::ReadWeights::F32(iter) => {
        //                         iter.enumerate().for_each(|(index, weight)| {
        //                             primitive.vertices[index].bone_weights = weight;
        //                         });
        //                     }
        //                     _ => return None,
        //                 }
        //             }

        //             Some(primitive)
        //         });

        //         if primitives.any(|primitive| primitive.is_none()) {
        //             Err(())
        //         } else {
        //             Ok(Mesh {
        //                 primitives: primitives.map(|primitive| primitive.unwrap()).collect(),
        //                 materials: vec![],
        //             })
        //         }
        //     })
        //     .collect::<Vec<_>>();

        // Ok(Scene {})

        Err(())
    }
}
