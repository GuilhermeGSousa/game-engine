use std::io::{BufReader, Cursor};

use super::{
    material::{self, Material},
    texture::Texture,
    vertex::Vertex,
    Mesh, SubMesh,
};
use async_trait::async_trait;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, utils::load_to_string, Asset,
    AssetPath,
};

pub(crate) struct ObjLoader;

#[async_trait]
impl AssetLoader for ObjLoader {
    type Asset = Mesh;

    async fn load(
        &self,
        path: AssetPath,
        load_context: &mut AssetLoadContext,
    ) -> Result<Self::Asset, ()> {
        let obj_text = load_to_string(path.clone()).await?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, materials) = tobj::load_obj_buf_async(
            &mut obj_reader,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            move |p| async move {
                let mat = load_to_string(AssetPath::new(p)).await.unwrap();
                let mat_cursor = Cursor::new(mat);
                tobj::load_mtl_buf(&mut BufReader::new(mat_cursor))
            },
        )
        .await
        .map_err(|_| ())?;

        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|vertex_index| {
                        let uv_coords = match m.mesh.texcoords.len() {
                            0 => [0.0, 0.0],
                            _ => [
                                m.mesh.texcoords[vertex_index * 2],
                                m.mesh.texcoords[vertex_index * 2 + 1],
                            ],
                        };

                        Vertex {
                            pos_coords: [
                                m.mesh.positions[vertex_index * 3],
                                m.mesh.positions[vertex_index * 3 + 1],
                                m.mesh.positions[vertex_index * 3 + 2],
                            ],
                            uv_coords: uv_coords,
                        }
                    })
                    .collect::<Vec<_>>();

                SubMesh {
                    vertices,
                    indices: m.mesh.indices,
                    material_index: 0,
                }
            })
            .collect::<Vec<_>>();

        let mesh_materials = materials
            .unwrap()
            .iter()
            .map(|m| {
                let mut material = Material::new();
                if let Some(diffuse_texture) = &m.diffuse_texture {
                    material.set_diffuse_texture(
                        load_context.asset_server().load::<Texture>(diffuse_texture),
                    )
                }
                material
            })
            .collect::<Vec<_>>();

        Ok(Mesh {
            meshes,
            materials: mesh_materials,
        })
    }
}
