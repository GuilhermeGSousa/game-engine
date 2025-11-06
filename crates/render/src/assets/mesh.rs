use essential::assets::Asset;

use super::vertex::Vertex;

#[derive(Asset)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}
