use anyhow::{anyhow, Result};
use wgpu::{ImageCopyTexture, ImageDataLayout, TextureAspect};

pub struct TextureReadback {
    staging_buffer: wgpu::Buffer,
    texture_extent: wgpu::Extent3d,
    bytes_per_row: u32,
}

impl TextureReadback {
    /// Create a new readback operation for a texture
    pub fn new(device: &wgpu::Device, texture: &wgpu::Texture) -> Result<Self> {
        let texture_extent = texture.size();

        // Calculate bytes needed for one row (must be padded to 256-byte alignment for GPU)
        let bytes_per_pixel = 4; // RGBA
        let bytes_per_row_unaligned = texture_extent.width * bytes_per_pixel;
        let bytes_per_row_padded = {
            let align = 256;
            ((bytes_per_row_unaligned + align - 1) / align) * align
        };

        let total_size = (bytes_per_row_padded * texture_extent.height) as u64;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Terminal Readback Staging Buffer"),
            size: total_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Self {
            staging_buffer,
            texture_extent,
            bytes_per_row: bytes_per_row_padded,
        })
    }

    /// Request readback of a texture to CPU
    pub fn request_readback(
        &self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });

        // Copy texture to staging buffer
        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.staging_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row),
                    rows_per_image: Some(self.texture_extent.height),
                },
            },
            self.texture_extent,
        );

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Poll for readback result (non-blocking, returns None if not ready)
    pub async fn poll_result(&self, device: &wgpu::Device) -> Result<Option<Vec<u8>>> {
        let buffer_slice = self.staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });

        // Poll for a short time (non-blocking for one frame)
        device.poll(wgpu::Maintain::Poll);

        // Try to receive immediately (don't block)
        match rx.try_recv() {
            Ok(Ok(())) => {
                // Buffer is mapped, read the data
                let data = buffer_slice.get_mapped_range();
                let result = data.to_vec();
                drop(data);
                self.staging_buffer.unmap();

                // Return only the relevant pixel data (remove padding)
                let bytes_per_pixel = 4;
                let width = self.texture_extent.width as usize;
                let height = self.texture_extent.height as usize;
                let bytes_per_row_unpadded = width * bytes_per_pixel;
                let bytes_per_row = self.bytes_per_row as usize;

                let mut pixel_data = Vec::with_capacity(width * height * bytes_per_pixel);
                for y in 0..height {
                    let row_start = y * bytes_per_row;
                    let row_end = row_start + bytes_per_row_unpadded;
                    pixel_data.extend_from_slice(&result[row_start..row_end]);
                }

                Ok(Some(pixel_data))
            }
            Ok(Err(e)) => Err(anyhow!("Map async error: {}", e)),
            Err(_) => Ok(None), // Not ready yet
        }
    }

    pub fn width(&self) -> u32 {
        self.texture_extent.width
    }

    pub fn height(&self) -> u32 {
        self.texture_extent.height
    }
}
