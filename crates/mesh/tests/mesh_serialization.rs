//! Covers Mesh/Vertex round-tripping through bincode directly (no DTO).
use mesh::mesh::Mesh;
use mesh::vertex::Vertex;

fn sample_vertex(x: f32) -> Vertex {
    Vertex {
        pos_coords: [x, 0.0, 0.0],
        uv_coords: [0.5, 0.5],
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        bitangent: [0.0, 0.0, 1.0],
        bone_indices: [0, 0, 0, 0],
        bone_weights: [1.0, 0.0, 0.0, 0.0],
    }
}

#[test]
fn mesh_round_trips_through_bincode_directly() {
    let mesh = Mesh {
        vertices: vec![sample_vertex(0.0), sample_vertex(1.0)],
        indices: vec![0, 1, 0],
    };

    let bytes = bincode::serialize(&mesh).expect("Mesh should serialize through bincode");
    let decoded: Mesh =
        bincode::deserialize(&bytes).expect("Mesh should deserialize back from bincode bytes");

    assert_eq!(
        decoded.indices, mesh.indices,
        "indices must survive a bincode round-trip unchanged"
    );
    assert_eq!(
        decoded.vertices[1].pos_coords,
        [1.0, 0.0, 0.0],
        "per-vertex position data must survive a bincode round-trip unchanged"
    );
}
