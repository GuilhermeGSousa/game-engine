use std::io::{BufReader, Cursor};

use super::{texture::Texture, vertex::Vertex, Mesh, SubMesh};
use async_trait::async_trait;
use essential::assets::{asset_loader::AssetLoader, utils::load_to_string, Asset, AssetPath};

pub(crate) struct ObjLoader;

#[async_trait]
impl AssetLoader for ObjLoader {
    type Asset = Mesh;

    async fn load(&self, path: AssetPath) -> Result<Self::Asset, ()> {
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
                    .map(|vertex_index| Vertex {
                        pos_coords: [
                            m.mesh.positions[vertex_index * 3],
                            m.mesh.positions[vertex_index * 3 + 1],
                            m.mesh.positions[vertex_index * 3 + 2],
                        ],
                        uv_coords: [0.0, 0.0],
                    })
                    .collect::<Vec<_>>();

                SubMesh {
                    vertices,
                    indices: m.mesh.indices,
                    material_index: 0,
                }
            })
            .collect::<Vec<_>>();

        let mesh_materials = Vec::new();
        // for m in materials.map_err(|_| ())? {
        //     if let Some(diffuse_path) = m.diffuse_texture {
        //         texture_loader.load(diffuse_path.into()); // shit
        //     }
        // }

        Ok(Mesh {
            meshes,
            materials: mesh_materials,
        })
    }
}
