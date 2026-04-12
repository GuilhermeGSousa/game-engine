use app::plugins::Plugin;
use glyphon::{Cache, FontSystem, SwashCache, Viewport};
use render::{
    assets::vertex::VertexBufferLayout, device::RenderDevice, material_plugin::MaterialPlugin,
    queue::RenderQueue, resources::RenderContext,
};
use wgpu::MultisampleState;

use crate::{
    checkbox::{UICheckboxChanged, sync_checkbox_material, toggle_checkboxes},
    focus::{FocusedWidget, update_focus},
    interaction::{HoveredNode, UIClick, apply_interaction_styles, update_ui_interaction},
    material::UIMaterial,
    node::{
        UIViewportPipeline, compute_ui_nodes, extract_added_ui_materials, extract_added_ui_nodes,
        extract_viewport_nodes, sync_border_size,
    },
    render::{prepare_text_renderer, ui_renderpass, update_text_viewport},
    slider::{UISliderChanged, setup_slider_visuals, sync_slider_fill, update_slider_drag},
    text::{
        extract_added_text_nodes,
        resources::{
            TextAtlas, TextCache, TextFontSystem, TextRenderer, TextSwashCache, TextViewport,
        },
    },
    text_input::{UITextInputChanged, update_text_inputs},
    vertex::UIVertex,
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_plugin(MaterialPlugin::<UIMaterial>::pipeline_only());

        // Resources
        app.insert_resource(HoveredNode(None));
        app.insert_resource(FocusedWidget(None));

        // Events
        app.register_event::<UIClick>();
        app.register_event::<UICheckboxChanged>();
        app.register_event::<UISliderChanged>();
        app.register_event::<UITextInputChanged>();

        // ── LateUpdate ──────────────────────────────────────────────────────────
        // 1. Spawn fill children for new sliders (commands flush immediately after).
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            setup_slider_visuals,
        );
        // 2. Update fill widths from slider values (before layout so Taffy sees
        //    the correct widths this frame).
        app.add_system(app::update_group::UpdateGroup::LateUpdate, sync_slider_fill);
        // 3. Taffy layout pass — computes UIComputedNode for all nodes.
        app.add_system(app::update_group::UpdateGroup::LateUpdate, compute_ui_nodes);
        // 4. Sync engine-managed border_params uniform from user-facing border_width.
        app.add_system(app::update_group::UpdateGroup::LateUpdate, sync_border_size);
        // 5. Slider drag — reads UIComputedNode set in step 3.
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            update_slider_drag,
        );
        // 6. Hit test — fires UIClick events.
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            update_ui_interaction,
        );
        // 7. Focus — reads HoveredNode, updates FocusedWidget.
        app.add_system(app::update_group::UpdateGroup::LateUpdate, update_focus);
        // 8. Checkbox toggle — reads UIClick.
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            toggle_checkboxes,
        );
        // 9. Sync checkbox material colour from checked state.
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            sync_checkbox_material,
        );
        // 10. Text input — reads FocusedWidget + typed chars, updates TextComponent.
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            update_text_inputs,
        );
        // 11. Hover/press colours via UIInteractionStyle.
        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            apply_interaction_styles,
        );

        // ── Render ──────────────────────────────────────────────────────────────
        app.add_system(
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
        // Viewport nodes: create fresh bind groups before ui_renderpass.
        .add_system(
            app::update_group::UpdateGroup::Render,
            extract_viewport_nodes,
        )
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

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);

        let mut atlas =
            glyphon::TextAtlas::new(device, queue, &cache, context.surface_config.format);
        let text_renderer =
            glyphon::TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        // ── UIViewportPipeline ──────────────────────────────────────────────────
        let viewport_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UIViewport BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let viewport_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UIViewport Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_viewport.wgsl").into()),
        });

        let viewport_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UIViewport Pipeline Layout"),
                bind_group_layouts: &[&viewport_bgl],
                push_constant_ranges: &[],
            });

        let vertex_layouts = UIVertex::describe();
        let viewport_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UIViewport Pipeline"),
            layout: Some(&viewport_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &viewport_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layouts],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &viewport_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        app.insert_resource(UIViewportPipeline {
            pipeline: viewport_pipeline,
            bind_group_layout: viewport_bgl,
        });

        app.insert_resource(TextRenderer(text_renderer))
            .insert_resource(TextCache(cache))
            .insert_resource(TextSwashCache(swash_cache))
            .insert_resource(TextViewport(viewport))
            .insert_resource(TextFontSystem(font_system))
            .insert_resource(TextAtlas(atlas));
    }
}
