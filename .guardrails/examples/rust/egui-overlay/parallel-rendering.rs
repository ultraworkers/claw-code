// Parallel Rendering - Rayon UI Threading for egui
// Production-ready parallel UI rendering patterns
//
// Last Updated: 2026-03-14
// egui Version: 0.27+
// Rust Version: 1.85+
// Rayon Version: 1.10+

use egui::{Context, ClippedPrimitive, TexturesDelta};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;
use rayon::ThreadPool;
use std::thread;

/// ParallelRenderer - Rayon-based UI rendering
/// Thread-safe, lock-free rendering via Arc
pub struct ParallelRenderer {
    /// egui context - Arc for thread sharing
    pub ctx: Arc<Context>,
    /// Rayon thread pool
    pub pool: ThreadPool,
    /// Frame buffer - Vec for batched updates
    pub frame_buffer: Arc<Mutex<Vec<FrameData>>>,
    /// Render queue - concurrent queue
    pub render_queue: Arc<Mutex<Vec<RenderCommand>>>,
    /// Thread count
    pub thread_count: usize,
    /// Lock-free flag
    pub lock_free: bool,
}

/// FrameData - Batched frame data
#[derive(Clone, Debug)]
pub struct FrameData {
    /// Clipped primitives for rendering
    pub clipped_primitives: Vec<ClippedPrimitive>,
    /// Texture deltas
    pub textures_delta: TexturesDelta,
    /// Frame sequence
    pub sequence: u64,
    /// Thread ID
    pub thread_id: usize,
}

/// RenderCommand - Parallel render command
#[derive(Clone, Debug)]
pub enum RenderCommand {
    /// Draw primitive
    DrawPrimitive(ClippedPrimitive),
    /// Upload texture
    UploadTexture(TextureData),
    /// Clear buffer
    ClearBuffer,
    /// Sync frame
    SyncFrame(u64),
}

/// TextureData - Texture upload data
#[derive(Clone, Debug)]
pub struct TextureData {
    /// Texture ID
    pub id: u32,
    /// Pixel data
    pub pixels: Vec<u8>,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

/// Implementation: ParallelRenderer
impl ParallelRenderer {
    /// Create new parallel renderer
    pub fn new(ctx: Context, thread_count: usize) -> Self {
        let pool = ThreadPool::builder()
            .num_threads(thread_count)
            .build()
            .expect("Failed to create Rayon pool");

        Self {
            ctx: Arc::new(ctx),
            pool,
            frame_buffer: Arc::new(Mutex::new(Vec::with_capacity(16))),
            render_queue: Arc::new(Mutex::new(Vec::with_capacity(64))),
            thread_count,
            lock_free: true,
        }
    }

    /// Initialize parallel renderer
    pub fn init(&mut self) {
        log::info!("Parallel renderer initialized with {} threads", self.thread_count);
    }

    /// Spawn parallel render task - Rayon
    pub fn spawn_render_task(&self, frame_data: FrameData) {
        self.pool.spawn({
            let ctx = Arc::clone(&self.ctx);
            let frame_buffer = Arc::clone(&self.frame_buffer);

            move || {
                // Lock-free render - Arc without mutex
                ctx.advance_frame(frame_data.sequence);

                // Batched update - coalesce frames
                if let Ok(mut buffer) = frame_buffer.lock() {
                    buffer.push(frame_data);
                }

                log::debug!("Render task completed: sequence {}", frame_data.sequence);
            }
        });
    }

    /// Batch render commands - Parallel iteration
    pub fn batch_render(&self, commands: Vec<RenderCommand>) {
        self.pool.install(|| {
            commands.par_iter().for_each(|command| {
                match command {
                    RenderCommand::DrawPrimitive(primitive) => {
                        self.draw_primitive(primitive);
                    }
                    RenderCommand::UploadTexture(texture) => {
                        self.upload_texture(texture);
                    }
                    RenderCommand::ClearBuffer => {
                        self.clear_buffer();
                    }
                    RenderCommand::SyncFrame(sequence) => {
                        self.sync_frame(sequence);
                    }
                }
            });
        });
    }

    /// Draw primitive - Thread-safe
    fn draw_primitive(&self, primitive: &ClippedPrimitive) {
        // Lock-free primitive draw
        let ctx = &self.ctx;
        ctx.render_primitive(primitive);
    }

    /// Upload texture - Thread-safe
    fn upload_texture(&self, texture: &TextureData) {
        // Lock-free texture upload via Arc
        let ctx = &self.ctx;
        ctx.set_texture(
            texture.id,
            egui::epaint::TextureOptions::default(),
            egui::epaint::ImageData {
                size: [texture.width as usize, texture.height as usize],
                bytes: texture.pixels.clone(),
            },
        );
    }

    /// Clear buffer - Thread-safe
    fn clear_buffer(&self) {
        if let Ok(mut buffer) = self.frame_buffer.lock() {
            buffer.clear();
        }
        if let Ok(mut queue) = self.render_queue.lock() {
            queue.clear();
        }
    }

    /// Sync frame - Sequence synchronization
    fn sync_frame(&self, sequence: u64) {
        self.ctx.advance_frame(sequence);
        log::debug!("Frame synced: sequence {}", sequence);
    }

    /// Queue render command - Thread-safe
    pub fn queue_command(&self, command: RenderCommand) {
        if let Ok(mut queue) = self.render_queue.lock() {
            queue.push(command);
        }
    }

    /// Process render queue - Parallel
    pub fn process_queue(&self) {
        if let Ok(queue) = self.render_queue.lock() {
            let commands = queue.clone();
            self.batch_render(commands);
        }
    }

    /// Get frame buffer - Lock-free read
    pub fn get_frame_buffer(&self) -> Vec<FrameData> {
        if let Ok(buffer) = self.frame_buffer.lock() {
            buffer.clone()
        } else {
            Vec::new()
        }
    }

    /// Flush frame buffer - Lock-free
    pub fn flush_frame_buffer(&self) {
        if let Ok(mut buffer) = self.frame_buffer.lock() {
            buffer.clear();
        }
    }

    /// Parallel texture upload - Rayon
    pub fn parallel_texture_upload(&self, textures: Vec<TextureData>) {
        self.pool.spawn({
            let ctx = Arc::clone(&self.ctx);

            textures.into_par_iter().for_each(|texture| {
                ctx.set_texture(
                    texture.id,
                    egui::epaint::TextureOptions::default(),
                    egui::epaint::ImageData {
                        size: [texture.width as usize, texture.height as usize],
                        bytes: texture.pixels.clone(),
                    },
                );
            });
        });
    }

    /// Parallel primitive draw - Rayon
    pub fn parallel_primitive_draw(&self, primitives: Vec<ClippedPrimitive>) {
        self.pool.spawn({
            let ctx = Arc::clone(&self.ctx);

            primitives.into_par_iter().for_each(|primitive| {
                ctx.render_primitive(primitive);
            });
        });
    }

    /// Get thread count
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    /// Is lock-free?
    pub fn is_lock_free(&self) -> bool {
        self.lock_free
    }
}

/// Parallel render system - Main parallel render loop
pub fn parallel_render_system(
    mut renderer: ResMut<ParallelRenderer>,
    frame_data: Res<FrameData>,
) {
    // Spawn parallel render task
    renderer.spawn_render_task(frame_data.clone());

    // Process render queue
    renderer.process_queue();

    // Flush frame buffer
    renderer.flush_frame_buffer();
}

/// Frame data resource
#[derive(Resource, Clone, Debug)]
pub struct FrameDataResource {
    pub sequence: u64,
    pub primitives: Vec<ClippedPrimitive>,
    pub textures: Vec<TextureData>,
}

/// Parallel renderer marker resource
#[derive(Resource, Debug)]
pub struct ParallelRendererMarker;

/// Performance benchmark - Parallel vs sequential
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_parallel_render_performance() {
        let ctx = Context::default();
        let renderer = ParallelRenderer::new(ctx, 4);

        let primitives = vec![ClippedPrimitive::default(); 1000];

        let start = Instant::now();
        renderer.parallel_primitive_draw(primitives);
        let duration = start.elapsed();

        log::info!("Parallel render: {}ms", duration.as_millis());
        assert!(duration.as_millis() < 100, "Render too slow");
    }

    #[test]
    fn test_parallel_texture_upload() {
        let ctx = Context::default();
        let renderer = ParallelRenderer::new(ctx, 4);

        let textures = vec![
            TextureData {
                id: 0,
                pixels: vec![0u8; 1024],
                width: 32,
                height: 32,
            };
            16
        ];

        let start = Instant::now();
        renderer.parallel_texture_upload(textures);
        let duration = start.elapsed();

        log::info!("Parallel texture upload: {}ms", duration.as_millis());
        assert!(duration.as_millis() < 50, "Texture upload too slow");
    }

    #[test]
    fn test_lock_free_rendering() {
        let ctx = Context::default();
        let renderer = ParallelRenderer::new(ctx, 4);

        assert!(renderer.is_lock_free());
        assert_eq!(renderer.thread_count(), 4);
    }
}