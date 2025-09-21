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
        let (document, buffers, images) = gltf::import(path.to_path()).unwrap();

        let meshes = document
            .meshes()
            .map(|mesh| {
                let primitives = mesh
                    .primitives()
                    .map(|gltf_primitive| {
                        let mut primitive = Primitive {
                            vertices: Vec::new(),
                            indices: Vec::new(),
                            material_index: 0,
                        };

                        let reader = gltf_primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                        primitive.indices = match reader.read_indices().ok_or(())? {
                            gltf::mesh::util::ReadIndices::U8(iter) => {
                                iter.map(|i| i as u32).collect()
                            }
                            gltf::mesh::util::ReadIndices::U16(iter) => {
                                iter.map(|i| i as u32).collect()
                            }
                            gltf::mesh::util::ReadIndices::U32(iter) => iter.collect(),
                        };

                        primitive.vertices = reader
                            .read_positions()
                            .ok_or(())?
                            .map(|pos| Vertex {
                                pos_coords: pos,
                                uv_coords: [0.0; 2],
                                normal: [0.0, 0.0, 1.0],
                                tangent: [0.0; 3],
                                bitangent: [0.0; 3],
                                bone_indices: [0; Vertex::MAX_AFFECTED_BONES],
                                bone_weights: [0.0; Vertex::MAX_AFFECTED_BONES],
                            })
                            .collect();

                        if let Some(uv_0) = reader.read_tex_coords(0) {
                            match uv_0 {
                                gltf::mesh::util::ReadTexCoords::F32(iter) => {
                                    iter.enumerate().for_each(|(index, uvs)| {
                                        primitive.vertices[index].uv_coords = uvs;
                                    });
                                }
                                _ => return Err(()),
                            }
                        }

                        if let Some(normals) = reader.read_normals() {
                            normals.enumerate().for_each(|(index, normal)| {
                                primitive.vertices[index].normal = normal;
                            });
                        }

                        if let Some(tangents) = reader.read_tangents() {
                            tangents.enumerate().for_each(|(index, tangent)| {
                                primitive.vertices[index].tangent =
                                    [tangent[0], tangent[1], tangent[2]];
                            });
                        }

                        if let Some(joints_0) = reader.read_joints(0) {
                            match joints_0 {
                                gltf::mesh::util::ReadJoints::U8(iter) => {
                                    iter.enumerate().for_each(|(index, joint)| {
                                        primitive.vertices[index].bone_indices = [
                                            joint[0].into(),
                                            joint[1].into(),
                                            joint[2].into(),
                                            joint[3].into(),
                                        ];
                                    })
                                }
                                gltf::mesh::util::ReadJoints::U16(iter) => {
                                    iter.enumerate().for_each(|(index, joint)| {
                                        primitive.vertices[index].bone_indices = [
                                            joint[0].into(),
                                            joint[1].into(),
                                            joint[2].into(),
                                            joint[3].into(),
                                        ];
                                    })
                                }
                            }
                        }

                        if let Some(weights_0) = reader.read_weights(0) {
                            match weights_0 {
                                gltf::mesh::util::ReadWeights::F32(iter) => {
                                    iter.enumerate().for_each(|(index, weight)| {
                                        primitive.vertices[index].bone_weights = weight;
                                    });
                                }
                                _ => return Err(()),
                            }
                        }

                        Ok::<Primitive, ()>(primitive)
                    })
                    .collect::<Vec<_>>();
            })
            .collect::<Vec<_>>();

        Ok(Scene {})
    }
}
