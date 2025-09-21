use async_trait::async_trait;
use essential::assets::asset_loader::AssetLoader;

use crate::assets::{
    mesh::{Mesh, Primitive},
    scene::Scene,
    vertex::Vertex,
};
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
            let submeshes = mesh.primitives().map(|primitive| {
                let mut submesh = Primitive {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    material_index: 0,
                };

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                submesh.indices = match reader.read_indices().ok_or(())? {
                    gltf::mesh::util::ReadIndices::U8(iter) => iter.map(|i| i as u32).collect(),
                    gltf::mesh::util::ReadIndices::U16(iter) => iter.map(|i| i as u32).collect(),
                    gltf::mesh::util::ReadIndices::U32(iter) => iter.collect(),
                };

                let positions = reader.read_positions().ok_or(())?;

                if let Some(uv_0) = reader.read_tex_coords(0) {}

                if let Some(normals) = reader.read_normals() {}

                if let Some(tangents) = reader.read_tangents() {}

                if let Some(joints_0) = reader.read_joints(0) {}

                // submesh.vertices = positions
                //     .map(|p| Vertex {
                //         pos_coords: todo!(),
                //         uv_coords: todo!(),
                //         normal: todo!(),
                //         tangent: todo!(),
                //         bitangent: todo!(),
                //         bone_indices: todo!(),
                //         bone_weights: todo!(),
                //     })
                //     .collect();
                Ok::<Primitive, ()>(submesh)
            });
        });

        Ok(Scene {})
    }
}
