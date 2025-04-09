// use core::assets::utils::load_string;
// use std::io::{BufReader, Cursor};

// use tobj::LoadOptions;

// use super::MeshAsset;

// struct ObjLoader {}

// impl ObjLoader {
//     async fn load_asset(&self, file_name: &str) -> anyhow::Result<MeshAsset> {
//         let obj_text = load_string(file_name).await?;
//         let obj_cursor = Cursor::new(obj_text);

//         let mut obj_reader = BufReader::new(obj_cursor);

//         let load_options = LoadOptions::default();
//         tobj::load_obj_buf(
//             &mut obj_reader,
//             &tobj::LoadOptions {
//                 triangulate: true,
//                 ..Default::default()
//             },
//             |path| {
//                 let mat_text = load_string(path.to_str().unwrap()).await.unwrap();
//                 tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
//             },
//         );

//         todo!("Implement OBJ loading logic here");
//     }
// }
