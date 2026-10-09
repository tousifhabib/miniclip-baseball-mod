//! Draws a frame's commands with wgpu.
//!
//! Meshes are built the first time a symbol is drawn and kept. Masks use the
//! stencil buffer: a mask's outline raises the stencil value of the pixels it
//! covers, and masked content only draws where the value matches.
//!
//! A blurred object is drawn to a layer of its own, a texture just big enough
//! to hold it. The layer is blurred and then drawn into its parent like any
//! other picture.
//!
//! A frame is drawn in three goes. Its commands are sorted into layers of
//! draws, in `plan`. What those need is handed to the graphics card, in
//! `upload`, onto textures kept in `targets`. Then the layers are drawn,
//! here. What the card is set up with to begin with is in `pipelines`,
//! finding and opening it is in `device`, and drawing to a picture in
//! memory is in `capture`.

mod capture;
mod device;
mod passes;
mod pipelines;
mod plan;
mod targets;
mod upload;

use std::collections::HashMap;

use bb_format::SymbolId;

use crate::input::Geometry;
use crate::library::Library;
use crate::meshes::{MeshKey, Meshes};
use pipelines::{Globals, Item, Layouts, Pipelines, Samplers};
use plan::Texture;
use targets::{LayerTarget, Targets};
use upload::{Buffers, ramp_texture, slot_buffer};

pub(crate) use device::open_device;

const SAMPLES: u32 = 4;
const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
/// Uniform data for one draw or one layer is placed at multiples of this, the
/// largest alignment any device asks for.
const SLOT: usize = 256;

/// What the last frame took to draw.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub draws: usize,
    pub layers: usize,
    pub meshes: usize,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipelines: Pipelines,
    layouts: Layouts,
    samplers: Samplers,
    /// What each layer is told, a slot to a layer.
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    globals_capacity: usize,
    /// What each draw is told, a slot to a draw.
    items: wgpu::Buffer,
    items_bind: wgpu::BindGroup,
    items_capacity: usize,
    /// The triangles of everything drawn or pointed at so far.
    meshes: Meshes,
    /// What the graphics card holds of each mesh that has been drawn.
    buffers: HashMap<MeshKey, Buffers>,
    ramps_bind: wgpu::BindGroup,
    ramps_texture: wgpu::Texture,
    ramps_capacity: usize,
    ramps_uploaded: usize,
    image_views: Vec<wgpu::TextureView>,
    image_binds: HashMap<Texture, wgpu::BindGroup>,
    targets: Option<Targets>,
    layer_targets: Vec<LayerTarget>,
    frame: u64,
    /// The least width a stroke is drawn at, in pixels of the target.
    pub min_stroke: f32,
    pub stats: Stats,
}

impl Renderer {
    /// `format` is the format of the textures this will draw to. It should not
    /// be an sRGB format: Flash blends colours as stored, without converting.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Renderer {
        let shader = pipelines::shader(&device);
        let layouts = Layouts::new(&device);
        let pipelines = Pipelines::new(&device, &shader, &layouts, format);
        let samplers = Samplers::new(&device);

        let globals_capacity = 16;
        let (globals, globals_bind) =
            slot_buffer::<Globals>(&device, &layouts.globals, globals_capacity);
        let items_capacity = 1024;
        let (items, items_bind) = slot_buffer::<Item>(&device, &layouts.item, items_capacity);
        let ramps_capacity = 64;
        let (ramps_texture, ramps_bind) =
            ramp_texture(&device, &layouts.texture, &samplers.ramp, ramps_capacity);

        Renderer {
            device,
            queue,
            format,
            pipelines,
            layouts,
            samplers,
            globals,
            globals_bind,
            globals_capacity,
            items,
            items_bind,
            items_capacity,
            meshes: Meshes::default(),
            buffers: HashMap::new(),
            ramps_bind,
            ramps_texture,
            ramps_capacity,
            ramps_uploaded: 0,
            image_views: Vec::new(),
            image_binds: HashMap::new(),
            targets: None,
            layer_targets: Vec::new(),
            frame: 0,
            min_stroke: 1.0,
            stats: Stats::default(),
        }
    }

    /// Takes up meshes that were built before there was a renderer, by a
    /// game that had only needed to know what was under its pointer. They
    /// go to the graphics card as each is first drawn.
    pub fn with_meshes(mut self, meshes: Meshes) -> Renderer {
        self.meshes = meshes;
        self
    }

    /// Symbols that could not be drawn, each reported once.
    pub fn problems(&self) -> &[String] {
        self.meshes.problems()
    }
}

impl Geometry for Renderer {
    fn contains(
        &mut self,
        library: &Library,
        symbol: SymbolId,
        ratio: u16,
        x: f32,
        y: f32,
    ) -> bool {
        self.meshes.contains(library, symbol, ratio, x, y)
    }
}
