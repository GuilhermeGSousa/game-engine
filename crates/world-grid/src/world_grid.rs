use color::LinearRgba;
use ecs::{
    component::{ComponentLifecycleCallback, ComponentLifecycleContext},
    Changed, Component, Query, ResMut,
};
use essential::{
    assets::{asset_server::AssetServer, asset_store::AssetStore},
    transform::Transform,
};
use mesh::{MeshComponent, Vertex};
use render::MaterialComponent;

use crate::material::{WorldGridMaterial, WorldGridUniform};

pub struct WorldGrid {
    pub cell_size: f32,
    pub coarse_cells: f32,
    pub line_color: LinearRgba,
    pub fade_start: f32,
    pub fade_end: f32,
    pub surface_color: LinearRgba,
}

impl Default for WorldGrid {
    fn default() -> Self {
        Self {
            cell_size: 1.0,
            coarse_cells: 10.0,
            line_color: LinearRgba::new(0.28, 0.28, 0.28, 0.85),
            fade_start: 80.0,
            fade_end: 200.0,
            surface_color: LinearRgba::WHITE,
        }
    }
}

impl Component for WorldGrid {
    fn name() -> &'static str {
        "WorldGrid"
    }

    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(world_grid_on_add)
    }

    fn on_remove() -> Option<ecs::component::ComponentLifecycleCallback> {
        None
    }
}

fn world_grid_on_add(
    mut world: ecs::world::RestrictedWorld<'_>,
    context: ComponentLifecycleContext,
) {
    let grid = world
        .get_component_for_entity::<WorldGrid>(context.entity)
        .unwrap();

    let uniform = WorldGridUniform {
        line_color: grid.line_color,
        cell_size: grid.cell_size,
        coarse_cells: grid.coarse_cells as f32,
        fade_start: grid.fade_start,
        fade_end: grid.fade_end,
        surface_color: grid.surface_color,
    };

    let (mesh_handle, material_handle) = {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mesh = render::assets::mesh::Mesh {
            vertices: vec![Vertex::default(); 3],
            indices: vec![0, 1, 2],
        };
        (
            asset_server.add(mesh),
            asset_server.add(WorldGridMaterial { uniform }),
        )
    };

    world.insert_component(
        MeshComponent {
            handle: mesh_handle,
        },
        context.entity,
        true,
    );
    world.insert_component(
        MaterialComponent::<WorldGridMaterial> {
            handle: material_handle,
        },
        context.entity,
        true,
    );
    world.insert_component(Transform::default(), context.entity, true);
}

pub(crate) fn on_world_grid_changed(
    query: Query<(&WorldGrid, &MaterialComponent<WorldGridMaterial>), Changed<WorldGrid>>,
    mut materials: ResMut<AssetStore<WorldGridMaterial>>,
) {
    query.iter().for_each(|(grid, material_component)| {
        let Some(material) = materials.get_mut(&material_component.handle) else {
            return;
        };

        material.uniform.line_color = grid.line_color;
        material.uniform.cell_size = grid.cell_size;
        material.uniform.coarse_cells = grid.coarse_cells;
        material.uniform.fade_start = grid.fade_start;
        material.uniform.fade_end = grid.fade_end;
        material.uniform.surface_color = grid.surface_color;
    });
}
