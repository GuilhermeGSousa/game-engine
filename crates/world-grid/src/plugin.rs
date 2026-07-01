use app::Plugin;
use color::LinearRgba;
use ecs::{command::CommandQueue, resource::Res, system::schedule::UpdateGroup};
use essential::assets::asset_server::AssetServer;
use mesh::MeshComponent;
use render::{assets::vertex::Vertex, MaterialComponent, MaterialPlugin};

use crate::material::{WorldGridMaterial, WorldGridUniform};

pub struct WorldGridPlugin {
    pub cell_size:    f32,
    pub coarse_cells: u32,
    pub line_color:   LinearRgba,
    pub fade_start:   f32,
    pub fade_end:     f32,
}

impl Default for WorldGridPlugin {
    fn default() -> Self {
        Self {
            cell_size:    1.0,
            coarse_cells: 10,
            line_color:   LinearRgba::new(0.28, 0.28, 0.28, 0.85),
            fade_start:   80.0,
            fade_end:     200.0,
        }
    }
}

impl Plugin for WorldGridPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_plugin(MaterialPlugin::<WorldGridMaterial>::new());

        let uniform = WorldGridUniform {
            line_color:   self.line_color,
            cell_size:    self.cell_size,
            coarse_cells: self.coarse_cells as f32,
            fade_start:   self.fade_start,
            fade_end:     self.fade_end,
            _padding:     [0; 4],
        };

        app.insert_resource(GridConfig(uniform));
        app.add_system(UpdateGroup::Startup, spawn_grid);
    }
}

struct GridConfig(WorldGridUniform);

impl ecs::resource::Resource for GridConfig {
    fn name() -> &'static str {
        "world_grid::plugin::GridConfig"
    }
}

fn spawn_grid(
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
    config: Res<GridConfig>,
) {
    let mesh = render::assets::mesh::Mesh {
        vertices: vec![Vertex::default(); 3],
        indices:  vec![0, 1, 2],
    };

    let mesh_handle     = asset_server.add(mesh);
    let material_handle = asset_server.add(WorldGridMaterial { uniform: config.0 });

    cmd.spawn((
        MeshComponent { handle: mesh_handle },
        MaterialComponent::<WorldGridMaterial> { handle: material_handle },
        essential::transform::Transform::default(),
    ));
}
