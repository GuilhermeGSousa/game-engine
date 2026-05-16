use crate::{
    AsciiConverter, ScreenBuffer, TerminalDevice, TextureReadbackResource,
};
use crossterm::style::SetForegroundColor;
use ecs::{
    query::Query,
    resource::{Res, ResMut},
    system::System,
};
use essential::assets::asset_store::AssetStore;
use render::{
    components::camera::Camera, device::RenderDevice, queue::RenderQueue,
    render_asset::render_texture::RenderTexture,
};
use std::io::Write;

pub fn terminal_output_system(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut terminal: ResMut<TerminalDevice>,
    mut readback_res: ResMut<TextureReadbackResource>,
    converter: Res<AsciiConverter>,
    render_textures: Res<AssetStore<RenderTexture>>,
) {
    // Get terminal dimensions
    let term_width = terminal.width();
    let term_height = terminal.height();

    // Update terminal size if needed
    let _ = terminal.update_size();

    // If we have previous pixel data, convert and display it
    if let Some(pixel_data) = readback_res.get_last_pixel_data() {
        let (last_width, last_height) = readback_res.get_last_dimensions();

        // Only render if dimensions match terminal
        if last_width as usize == term_width && last_height as usize == term_height {
            let screen_grid = converter.pixels_to_screen(pixel_data, term_width, term_height);

            // Clear terminal
            let _ = terminal.clear();

            // Write to terminal using ANSI codes
            let mut stdout = std::io::stdout();
            for (y, row) in screen_grid.iter().enumerate() {
                // Move cursor to start of line
                let _ = write!(stdout, "\x1b[{};0H", y + 1);

                for (x, (ch, color_code)) in row.iter().enumerate() {
                    // Set color and write character
                    let color_seq = format!("\x1b[38;5;{}m", color_code);
                    let _ = write!(stdout, "{}{}", color_seq, ch);
                }
            }

            // Reset color
            let _ = write!(stdout, "\x1b[0m");
            let _ = stdout.flush();
        }
    }

    // If we have a readback result pending, poll it (would need async handling)
    // For now, we'll just set up the next frame's readback request

    // Try to get render texture and request readback
    if let Some(texture_handle) = readback_res.terminal_render_target() {
        if let Some(render_texture) = render_textures.get(texture_handle) {
            // Create new readback for this texture
            if let Ok(readback) = crate::readback::TextureReadback::new(
                &device,
                &render_texture.texture,
            ) {
                let width = readback.width();
                let height = readback.height();

                // Request readback
                let _ = readback.request_readback(&device, &render_texture.texture, &queue);

                readback_res.set_readback(readback);
            }
        }
    }
}
