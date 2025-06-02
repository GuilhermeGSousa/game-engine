use std::io::{BufRead, BufReader, Cursor};

use async_trait::async_trait;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, utils::load_to_string, AssetPath,
};

use crate::assets::{
    material::Material,
    mesh::{Mesh, SubMesh},
    vertex::Vertex,
};

pub(crate) struct ObjLoader;

#[async_trait]
#[allow(deprecated)]
impl AssetLoader for ObjLoader {
    type Asset = Mesh;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
    ) -> Result<Self::Asset, ()> {
        let obj_text = load_to_string(path.clone()).await?;
        let obj_cursor = Cursor::new(obj_text);

        let mat_handles = BufReader::new(obj_cursor.clone())
            .lines()
            .filter_map(Result::ok)
            .filter_map(|line| {
                if line.starts_with("mtllib") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        let mtl_path = path.to_path().parent().unwrap().join(parts[1]);
                        Some(load_context.asset_server().load::<Material>(mtl_path))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let (models, _) = tobj::load_obj_buf_async(
            &mut BufReader::new(obj_cursor),
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            move |_| async move { Err(tobj::LoadError::GenericFailure) },
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

        Ok(Mesh {
            meshes,
            materials: mat_handles,
        })
    }
}
