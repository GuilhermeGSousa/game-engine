use app::plugins::Plugin;
use glyphon::{Cache, FontSystem, SwashCache, Viewport};
use render::{
    device::RenderDevice,
    material_plugin::MaterialPlugin,
    queue::RenderQueue,
    resources::RenderContext,
};
use wgpu::MultisampleState;

use crate::{
    material::UIMaterial,
    node::{compute_ui_nodes, extract_added_ui_materials, extract_added_ui_nodes},
    render::{prepare_text_renderer, ui_renderpass, update_text_viewport},
    text::{
        extract_added_text_nodes,
        resources::{
            TextAtlas, TextCache, TextFontSystem, TextRenderer, TextSwashCache, TextViewport,
        },
    },
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_plugin(MaterialPlugin::<UIMaterial>::pipeline_only());
        app.add_system(app::update_group::UpdateGroup::LateUpdate, compute_ui_nodes)
            .add_system(
                app::update_group::UpdateGroup::Render,
                extract_added_ui_nodes,
            )
            .add_system(
                app::update_group::UpdateGroup::Render,
                extract_added_ui_materials,
            )
            .add_system(
                app::update_group::UpdateGroup::Render,
                extract_added_text_nodes,
            )
            .add_system(app::update_group::UpdateGroup::Render, update_text_viewport)
            .add_system(
                app::update_group::UpdateGroup::Render,
                prepare_text_renderer,
            )
            // The UI pass must run in LateRender so it draws on top of all 3-D
            // scene passes (skybox, main mesh, custom material passes) which are
            // scheduled in the Render group.
            .add_system(app::update_group::UpdateGroup::Render, ui_renderpass);
    }

    fn finish(&self, app: &mut app::App) {
        let device = app
            .get_resource::<RenderDevice>()
            .expect("RenderDevice resource not found");

        let context = app
            .get_resource::<RenderContext>()
            .expect("RenderContext resource not found");

        let queue = app
            .get_resource::<RenderQueue>()
            .expect("RenderQueue resource not found");

        // Text rendering
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        // TODO: Update Viewport on resize!

        let mut atlas =
            glyphon::TextAtlas::new(&device, &queue, &cache, context.surface_config.format);
        let text_renderer =
            glyphon::TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

        app.insert_resource(TextRenderer(text_renderer))
            .insert_resource(TextCache(cache))
            .insert_resource(TextSwashCache(swash_cache))
            .insert_resource(TextViewport(viewport))
            .insert_resource(TextFontSystem(font_system))
            .insert_resource(TextAtlas(atlas));
    }
}
