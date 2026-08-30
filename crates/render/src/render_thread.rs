//! Pipelined rendering: run the render world's schedules on a dedicated thread,
//! one frame behind the main-world simulation.
//!
//! Modelled on Bevy's `PipelinedRenderingPlugin`. [`RenderThreadPlugin`] pulls
//! the [`RenderApp`](app::subapp::RenderApp) sub-app out of the [`App`] and hands
//! it to a render thread. Each frame, a stand-in `RenderExtractApp` sub-app runs
//! on the main thread: it reclaims the render sub-app from the thread, runs the
//! (main-thread-only) extract step, then sends it back to render while the main
//! thread proceeds to the next frame's simulation.

use app::{App, Plugin};

/// Runs the render world's schedules on a dedicated thread, pipelined one frame
/// behind the main-world simulation (à la Bevy's `PipelinedRenderingPlugin`).
///
/// Register it **last** — after [`RenderPlugin`](crate::plugin::RenderPlugin) and
/// anything else that touches the render sub-app (it removes that sub-app in
/// [`Plugin::finish`]). Rendered output lags simulation by one frame.
///
/// No-op on wasm, which has no threads; rendering stays inline there.
pub struct RenderThreadPlugin;

impl Plugin for RenderThreadPlugin {
    fn build(&self, _app: &mut App) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn finish(&self, app: &mut App) {
        pipelined::install(app);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod pipelined {
    use std::{
        sync::mpsc::{self, Receiver, Sender},
        thread::JoinHandle,
    };

    use app::{
        schedule_groups::Startup,
        subapp::{RenderApp, SubApp, SubAppLabel},
        App,
    };
    use ecs::World;

    /// Label of the stand-in sub-app that replaces [`RenderApp`] on the main
    /// thread once rendering is pipelined.
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct RenderExtractApp;

    impl SubAppLabel for RenderExtractApp {
        fn dyn_clone(&self) -> Box<dyn SubAppLabel> {
            Box::new(self.clone())
        }
    }

    /// Channels + handle to the render thread. Captured by the
    /// `RenderExtractApp` extract closure, which is the per-frame sync point.
    struct RenderThreadBridge {
        /// `None` only while [`Drop`] disconnects the channel.
        to_render: Option<Sender<SubApp>>,
        from_render: Receiver<SubApp>,
        /// Holds the render sub-app until the first extract hands it over.
        pending: Option<SubApp>,
        /// Whether the render thread currently owns the render sub-app.
        in_flight: bool,
        handle: Option<JoinHandle<()>>,
    }

    impl Drop for RenderThreadBridge {
        fn drop(&mut self) {
            // Dropping the sender disconnects the channel; the render thread's
            // `recv()` then returns `Err` and its loop exits. `from_render`
            // outlives this body, so a frame still in flight is delivered (and
            // dropped) rather than lost.
            drop(self.to_render.take());
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn render_thread_main(from_main: Receiver<SubApp>, to_main: Sender<SubApp>) {
        profiling::register_thread!("render");
        while let Ok(mut render_app) = from_main.recv() {
            {
                profiling::scope!("render_thread::update");
                render_app.update();
            }
            if to_main.send(render_app).is_err() {
                break;
            }
        }
    }

    pub(super) fn install(app: &mut App) {
        let mut render_app = app.remove_sub_app(RenderApp).expect(
            "RenderApp sub-app missing; register RenderPlugin before RenderThreadPlugin",
        );

        // `finish_plugin_build` compiles sub-app schedules and runs `Startup`
        // *after* the plugin `finish()` pass — by which point this sub-app is no
        // longer in the `App`. Do that work here before handing it off.
        render_app.compile_schedules();
        render_app.world_mut().run_schedule(Startup);

        let (to_render, render_rx) = mpsc::channel::<SubApp>();
        let (to_main, main_rx) = mpsc::channel::<SubApp>();

        let handle = std::thread::Builder::new()
            .name("render".into())
            .spawn(move || render_thread_main(render_rx, to_main))
            .expect("failed to spawn render thread");

        let mut bridge = RenderThreadBridge {
            to_render: Some(to_render),
            from_render: main_rx,
            pending: Some(render_app),
            in_flight: false,
            handle: Some(handle),
        };

        let mut extract_app = SubApp::default();
        extract_app.set_extract(Box::new(move |main_world: &mut World, _shim: &mut World| {
            profiling::scope!("pipelined_render::extract");

            // Reclaim the render sub-app: block on the thread once one is in
            // flight (the pipeline stalls here only if rendering is slower than
            // simulation); use the stashed one on the first frame.
            let mut render_app = if bridge.in_flight {
                profiling::scope!("await::render_thread");
                bridge
                    .from_render
                    .recv()
                    .expect("render thread terminated unexpectedly")
            } else {
                bridge.pending.take().expect("render sub-app already taken")
            };

            // Extract runs on the main thread — the one place both worlds meet.
            render_app.run_extract(main_world);

            bridge
                .to_render
                .as_ref()
                .expect("render bridge sender missing")
                .send(render_app)
                .expect("render thread terminated unexpectedly");
            bridge.in_flight = true;
        }));

        app.insert_sub_app(RenderExtractApp, extract_app);
    }
}
