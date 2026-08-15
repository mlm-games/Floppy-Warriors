use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use renamite_model::{Color, Document, NodeKind, StylePaint};
use renamite_render_bridge::SceneRenderer;
use renamite_render_offscreen::{OffscreenRenderer, fit_view};

/// Body-part textures rasterized from the SVGs in `assets/images/`.
/// Sprites reference these handles, then set `custom_size` for scale and
/// `color` for team/archetype tinting.
#[derive(Resource, Clone)]
pub struct WarriorArt {
    pub body: Handle<Image>,
    pub head: Handle<Image>,
    pub upper_arm: Handle<Image>,
    pub hand: Handle<Image>,
    pub leg: Handle<Image>,
    pub bow: Handle<Image>,
    /// Full-bleed landscape backdrop (title + gameplay).
    pub bg: Handle<Image>,
    /// Full tileset atlas (no alpha-trim). 32 columns × 16 rows of 16px cells
    /// (Godot's atlas layout; our bake is 4x, so cells are 64px there).
    pub tileset: Handle<Image>,
    /// Pixel size of one cell in `tileset` (width / 8).
    pub tile_px: u32,
}

struct SvgBaker {
    gpu: OffscreenRenderer,
    bridge: SceneRenderer,
    canvas: u32,
}

const CANVAS: u32 = 2048;
/// Tileset layout contract (must match tileset.svg).
const TILESET_COLS: u32 = 32;
const TILESET_ROWS: u32 = 16;

impl SvgBaker {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            gpu: OffscreenRenderer::new_blocking(CANVAS, CANVAS, 1)?,
            bridge: SceneRenderer::new(),
            canvas: CANVAS,
        })
    }

    fn import_doc(svg_path: &str) -> anyhow::Result<Document> {
        let bytes = std::fs::read(svg_path).map_err(|e| anyhow::anyhow!("read {svg_path}: {e}"))?;
        let report = renamite_io_svg::import_with_report(&bytes)
            .map_err(|e| anyhow::anyhow!("import {svg_path}: {e}"))?;
        for w in &report.warnings {
            bevy::log::warn!("SVG warning [{}] {}: {}", svg_path, w.path, w.message);
        }
        Ok(report.value)
    }

    fn render_doc(
        &mut self,
        doc: Document,
        label: &str,
    ) -> anyhow::Result<(Vec<u8>, u32, u32, u32, u32)> {
        let (aw, ah) = doc.compositions[doc.main].size;
        anyhow::ensure!(aw > 0 && ah > 0, "{label}: empty artboard size");

        let view = fit_view((aw, ah), self.canvas, self.canvas);
        let project = renamite_io_ren::RenFile::new(doc, label);
        let mut player = renamite_player::Player::new(project)?;
        player.engine.scrub(&player.project, 0.0);
        self.gpu.sync_document_images(&player.project.document)?;

        let prepared = self.bridge.prepare(player.engine.scene(), &view);
        let mut repose_scene = repose_core::Scene::default();
        self.bridge
            .append_repose_scene(&prepared, &mut repose_scene);

        // Transparent clear so trim / empty detection works.
        let rgba = self
            .gpu
            .render_rgba(&repose_scene, Some([0.0, 0.0, 0.0, 0.0]))?;

        let ox = view.offset.x.round().max(0.0) as u32;
        let oy = view.offset.y.round().max(0.0) as u32;
        let pw = ((aw as f64) * view.scale).round().max(1.0) as u32;
        let ph = ((ah as f64) * view.scale).round().max(1.0) as u32;
        let pw = pw.min(self.canvas.saturating_sub(ox));
        let ph = ph.min(self.canvas.saturating_sub(oy));

        Ok((rgba, ox, oy, pw, ph))
    }

    /// Rasterize + alpha-trim (body parts).
    fn bake(&mut self, svg_path: &str, tintable: bool) -> anyhow::Result<Image> {
        let mut doc = Self::import_doc(svg_path)?;
        if tintable {
            grayscale_paints(&mut doc);
        }
        let (rgba, _ox, _oy, _pw, _ph) = self.render_doc(doc, "part")?;
        // Full canvas trim — content may sit anywhere after fit_view letterbox.
        trim_to_image(&rgba, self.canvas, self.canvas)
    }

    /// Full artboard, no trim (bg + atlas). Crop is the letterboxed artboard rect.
    fn bake_artboard(&mut self, svg_path: &str) -> anyhow::Result<(Image, u32, u32)> {
        let doc = Self::import_doc(svg_path)?;
        let (rgba, ox, oy, pw, ph) = self.render_doc(doc, "artboard")?;

        let mut data = Vec::with_capacity((pw * ph * 4) as usize);
        let w = self.canvas as usize;
        let mut any = false;
        for y in 0..ph as usize {
            for x in 0..pw as usize {
                let i = ((oy as usize + y) * w + (ox as usize + x)) * 4;
                let a = rgba[i + 3] as f32 / 255.0;
                if a > 0.0 {
                    any = true;
                    let cv = |v: u8| ((v as f32 / 255.0 / a).min(1.0) * 255.0) as u8;
                    data.extend([cv(rgba[i]), cv(rgba[i + 1]), cv(rgba[i + 2]), rgba[i + 3]]);
                } else {
                    data.extend([0, 0, 0, 0]);
                }
            }
        }
        anyhow::ensure!(any, "{svg_path}: artboard rasterized fully transparent");

        Ok((image_rgba8(pw, ph, data, ImageSampler::Default), pw, ph))
    }
}

pub fn bake_warrior_art(images: &mut Assets<Image>) -> anyhow::Result<WarriorArt> {
    let mut baker = SvgBaker::new()?;

    let (mut tileset_img, _atlas_w, atlas_h, tile_px) = match baker
        .bake_artboard("assets/images/tileset.svg")
    {
        Ok((img, w, h)) => {
            let tp = (w / TILESET_COLS).max(1);
            if w % TILESET_COLS != 0 || h % TILESET_ROWS != 0 {
                bevy::log::warn!(
                    "tileset.svg artboard {w}x{h} is not divisible by {TILESET_COLS}x{TILESET_ROWS}; tile_px={tp}"
                );
            }
            (img, w, h, tp)
        }
        Err(e) => {
            bevy::log::error!("tileset bake failed ({e}); using procedural fallback");
            let (img, tp) = procedural_tileset();
            (img, tp * TILESET_COLS, tp * TILESET_ROWS, tp)
        }
    };
    let _ = atlas_h; // kept for future asserts
    tileset_img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::nearest());

    let bg_img = match baker.bake_artboard("assets/images/bg.svg") {
        Ok((img, _, _)) => img,
        Err(e) => {
            bevy::log::error!("bg bake failed ({e}); using procedural fallback");
            procedural_bg()
        }
    };

    let mut bake = |svg_path: &str, tintable: bool| -> anyhow::Result<Handle<Image>> {
        Ok(images.add(baker.bake(svg_path, tintable)?))
    };

    Ok(WarriorArt {
        body: bake("assets/images/body.svg", true)?,
        head: bake("assets/images/head.svg", true)?,
        upper_arm: bake("assets/images/upper_arm.svg", true)?,
        hand: bake("assets/images/hand.svg", true)?,
        leg: bake("assets/images/leg.svg", true)?,
        bow: bake("assets/images/bow.svg", false)?,
        bg: images.add(bg_img),
        tileset: images.add(tileset_img),
        tile_px,
    })
}

// ---------------------------------------------------------------------------
// Procedural fallbacks — game must never boot with blank arena art.
// ---------------------------------------------------------------------------

fn procedural_bg() -> Image {
    // 16:9-ish solid sky + ground band.
    let w = 320u32;
    let h = 180u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let (r, g, b) = if y > (h * 5 / 6) {
                (200, 115, 55) // dirt shelf
            } else if y > (h * 2 / 3) {
                (100, 160, 110) // meadow
            } else {
                // sky lerp
                let t = y as f32 / (h as f32 * 0.66);
                let r = (126.0 + t * 40.0) as u8;
                let g = (184.0 + t * 30.0) as u8;
                let b = (232.0 - t * 20.0) as u8;
                (r, g, b)
            };
            data[i] = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = 255;
        }
    }
    image_rgba8(w, h, data, ImageSampler::Default)
}

fn procedural_tileset() -> (Image, u32) {
    let tp = 32u32;
    let w = tp * TILESET_COLS;
    let h = tp * TILESET_ROWS;
    let mut data = vec![0u8; (w * h * 4) as usize];

    let put = |data: &mut [u8], col: u32, row: u32, rgb: [u8; 3]| {
        let x0 = col * tp;
        let y0 = row * tp;
        for yy in 0..tp {
            for xx in 0..tp {
                let i = (((y0 + yy) * w + (x0 + xx)) * 4) as usize;
                data[i] = rgb[0];
                data[i + 1] = rgb[1];
                data[i + 2] = rgb[2];
                data[i + 3] = 255;
            }
        }
    };

    // (0,1) surface
    put(&mut data, 0, 1, [200, 113, 55]);
    // lighter stone cap on top half of cell
    {
        let x0 = 0u32 * tp + 4;
        let y0 = 1u32 * tp + 4;
        for yy in 0..(tp / 2) {
            for xx in 0..(tp - 8) {
                let i = (((y0 + yy) * w + (x0 + xx)) * 4) as usize;
                data[i] = 232;
                data[i + 1] = 168;
                data[i + 2] = 120;
                data[i + 3] = 255;
            }
        }
    }
    // (1,1) fill
    put(&mut data, 1, 1, [176, 90, 40]);
    // deeper rows
    put(&mut data, 0, 2, [138, 68, 32]);
    put(&mut data, 1, 2, [122, 60, 28]);

    let img = image_rgba8(
        w,
        h,
        data,
        ImageSampler::Descriptor(ImageSamplerDescriptor::nearest()),
    );
    (img, tp)
}

fn image_rgba8(width: u32, height: u32, data: Vec<u8>, sampler: ImageSampler) -> Image {
    let mut img = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    img.sampler = sampler;
    img
}

fn grayscale_paints(doc: &mut Document) {
    for node in doc.nodes.values_mut() {
        if let NodeKind::Style(style) = &mut node.kind {
            let gray = gray_paint(style.paint());
            let _ = style.swap_paint(gray);
        }
    }
}

fn gray_paint(paint: &StylePaint) -> StylePaint {
    let lum = |c: Color| {
        let l = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
        Color {
            r: l,
            g: l,
            b: l,
            a: c.a,
        }
    };
    match paint {
        StylePaint::Solid { .. } => StylePaint::solid(lum(paint.base_color())),
        StylePaint::Gradient(g) => {
            let mut out = g.clone();
            for stop in out.stops.base.0.iter_mut() {
                stop.color = lum(stop.color);
            }
            StylePaint::Gradient(out)
        }
    }
}

fn trim_to_image(rgba: &[u8], width: u32, height: u32) -> anyhow::Result<Image> {
    let (w, h) = (width as usize, height as usize);
    let (mut min_x, mut min_y) = (w, h);
    let (mut max_x, mut max_y) = (0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if rgba[(y * w + x) * 4 + 3] != 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    anyhow::ensure!(max_x >= min_x && max_y >= min_y, "SVG rasterized empty");
    let (cw, ch) = (max_x - min_x + 1, max_y - min_y + 1);
    let mut data = Vec::with_capacity(cw * ch * 4);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let i = (y * w + x) * 4;
            let a = rgba[i + 3] as f32 / 255.0;
            if a > 0.0 {
                let cv = |v: u8| ((v as f32 / 255.0 / a).min(1.0) * 255.0) as u8;
                data.extend([cv(rgba[i]), cv(rgba[i + 1]), cv(rgba[i + 2]), rgba[i + 3]]);
            } else {
                data.extend([0, 0, 0, 0]);
            }
        }
    }
    Ok(image_rgba8(
        cw as u32,
        ch as u32,
        data,
        ImageSampler::Default,
    ))
}
