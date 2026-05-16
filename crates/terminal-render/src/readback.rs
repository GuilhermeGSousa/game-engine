use anyhow::{anyhow, Result};

pub struct TextureReadback {
    staging_buffer: wgpu::Buffer,
    texture_extent: wgpu::Extent3d,
    bytes_per_row: u32,
}

impl TextureReadback {
    pub fn new(device: &wgpu::Device, texture: &wgpu::Texture) -> Result<Self> {
        let texture_extent = texture.size();

        let bytes_per_pixel = 4; // RGBA
        let bytes_per_row_unaligned = texture_extent.width * bytes_per_pixel;
        // wgpu requires rows aligned to 256 bytes
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes_per_row = (bytes_per_row_unaligned + align - 1) / align * align;

        let total_size = (bytes_per_row * texture_extent.height) as u64;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Terminal Readback Staging Buffer"),
            size: total_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Self {
            staging_buffer,
            texture_extent,
            bytes_per_row,
        })
    }

    /// Copy a texture to the staging buffer and block until data is ready.
    pub fn read_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
    ) -> Result<Vec<u8>> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row),
                    rows_per_image: Some(self.texture_extent.height),
                },
            },
            self.texture_extent,
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer and wait for the GPU to finish
        let buffer_slice = self.staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });

        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| anyhow!("Channel recv error: {}", e))?
            .map_err(|e| anyhow!("Buffer map error: {}", e))?;

        // Strip row padding and return tightly-packed RGBA pixels
        let data = buffer_slice.get_mapped_range();
        let width = self.texture_extent.width as usize;
        let height = self.texture_extent.height as usize;
        let bytes_per_pixel = 4;
        let padded_row = self.bytes_per_row as usize;
        let tight_row = width * bytes_per_pixel;

        let mut pixels = Vec::with_capacity(width * height * bytes_per_pixel);
        for y in 0..height {
            let start = y * padded_row;
            pixels.extend_from_slice(&data[start..start + tight_row]);
        }

        drop(data);
        self.staging_buffer.unmap();

        Ok(pixels)
    }

    pub fn width(&self) -> u32 {
        self.texture_extent.width
    }

    pub fn height(&self) -> u32 {
        self.texture_extent.height
    }
}
