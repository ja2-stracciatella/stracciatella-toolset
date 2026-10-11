//! The data of the replay shader: sprites as palette indices, palettes, draws and bins
//!
//! Everything is packed into flat `u32` arrays that become storage buffers. Only the sprites and
//! palettes that the draw list uses are included.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    render::{
        bins::Bins,
        draw_list::{Draw, DrawSprite, PaletteKey, SpriteGeometry, SpriteKey},
        palette::{pack_rgba, shaded_palettes, shadow_table, SHADE_LEVELS},
    },
    tileset::StciSheet,
    tileset::Tileset,
};

/// Number of `u32` words of a draw in the draw buffer
pub const DRAW_WORDS: usize = 7;
/// Number of `u32` words of a sprite in the sprite buffer
pub const SPRITE_WORDS: usize = 3;
/// Marks the sprite word of a draw as an animation id
pub const ANIMATED_SPRITE: u32 = 0x8000_0000;

/// The images and palettes that draws can refer to
pub struct SpriteSheets<'a> {
    pub tileset: &'a Tileset,
    /// The loaded sheet of each tileset surface, indexed by `SurfaceId`
    pub surface_sheets: Vec<Option<&'a StciSheet>>,
    /// Animation surfaces of soldiers, see `SpriteKey::Soldier`
    pub soldier_sheets: Vec<Option<&'a StciSheet>>,
    /// Palettes of soldiers, see `PaletteKey::Soldier`
    pub soldier_palettes: Vec<[[u8; 3]; 256]>,
}

impl SpriteSheets<'_> {
    /// The sheet and sub image index of a sprite
    fn sub_image(&self, key: SpriteKey) -> Option<(&StciSheet, usize)> {
        match key {
            SpriteKey::Tile(index) => {
                let image = &self.tileset.tile(index)?.image;
                let sheet = (*self.surface_sheets.get(image.surface.id.index())?)?;
                Some((sheet, usize::from(image.sub_image)))
            }
            SpriteKey::Soldier { surface, frame } => {
                let sheet = (*self.soldier_sheets.get(usize::from(surface))?)?;
                Some((sheet, usize::from(frame)))
            }
        }
    }

    /// Offset and size of a sprite, `None` if it can't be drawn
    pub fn geometry(&self, key: SpriteKey) -> Option<SpriteGeometry> {
        let (sheet, index) = self.sub_image(key)?;
        self.pixels(key)?;
        let info = sheet.sub_images.get(index)?;
        Some(SpriteGeometry {
            offset: IVec2::new(info.offset.0.into(), info.offset.1.into()),
            size: UVec2::new(info.dimensions.0.into(), info.dimensions.1.into()),
        })
    }

    /// The palette indices of a sprite, row by row
    pub fn pixels(&self, key: SpriteKey) -> Option<&[u8]> {
        let (sheet, index) = self.sub_image(key)?;
        let pixels = sheet.indexed.as_ref()?.pixels.get(index)?;
        let (width, height) = sheet.sub_images.get(index)?.dimensions;
        let size = usize::from(width) * usize::from(height);
        (size > 0 && pixels.len() == size).then_some(pixels.as_slice())
    }

    pub fn palette(&self, key: PaletteKey) -> Option<&[[u8; 3]; 256]> {
        match key {
            PaletteKey::Tile(surface) => {
                let sheet = (*self.surface_sheets.get(surface.index())?)?;
                Some(&sheet.indexed.as_ref()?.palette)
            }
            PaletteKey::Soldier(id) | PaletteKey::SoldierUnbiased(id) => {
                self.soldier_palettes.get(usize::from(id))
            }
        }
    }
}

/// Values of the replay shader that are not in storage buffers
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SceneParams {
    pub bins_origin: IVec2,
    pub bins_count: UVec2,
    /// Start of the draw indices in the bins buffer
    pub bin_indices_offset: u32,
    /// Start of the animation table in the sprite buffer
    pub animations_offset: u32,
}

/// The sprites, palettes and animations that the draws of a scene refer to
///
/// Only grows: draws that were packed once stay valid when more draws are packed later, so the
/// draws of a scene can be packed in parts. `version` changes whenever something is added.
#[derive(Clone, Debug, Default)]
pub struct SceneAtlas {
    /// Added to all palettes (`g_light_color`)
    light: [u8; 3],
    /// Palette indices of the sprites, 4 per word, little endian
    atlas: Vec<u32>,
    atlas_len: usize,
    /// `SPRITE_WORDS` per sprite
    sprites: Vec<u32>,
    sprite_slots: HashMap<SpriteKey, Option<u32>>,
    /// Rows of 256 colours: the shadow table, then the used palettes and shade levels
    palettes: Vec<u32>,
    /// First row of the shade levels of each palette
    palette_rows: HashMap<PaletteKey, u32>,
    /// Sprite indices of the frames of each animation
    animations: Vec<Vec<u32>>,
    /// Tile database index of each animation
    animation_tiles: Vec<u16>,
    animation_slots: HashMap<u16, u32>,
    pub version: u64,
}

/// The buffers of a `SceneAtlas`, shared by all draws packed with it
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SharedBuffers {
    pub atlas: Vec<u32>,
    pub palettes: Vec<u32>,
    /// `SPRITE_WORDS` per sprite, then per animation the start and number of its frames and
    /// finally the sprite indices of the frames
    pub sprites: Vec<u32>,
    /// Start of the animation table in `sprites`
    pub animations_offset: u32,
    /// Tile database index of each animation, to update the current frames
    pub animations: Vec<u16>,
}

impl SceneAtlas {
    pub fn new(light: [u8; 3]) -> Self {
        Self {
            light,
            palettes: shadow_table().to_vec(),
            ..default()
        }
    }

    /// Packs draws for the draw buffer, adding what they need to the atlas
    pub fn pack_draws(&mut self, draws: &[Draw], sheets: &SpriteSheets) -> Vec<u32> {
        let mut packed = Vec::with_capacity(draws.len() * DRAW_WORDS);
        for draw in draws {
            let sprite = match draw.sprite {
                DrawSprite::Fixed(key) => self.sprite(key, sheets).unwrap_or(0),
                DrawSprite::Animated(animation) => {
                    ANIMATED_SPRITE | self.animation(animation, sheets)
                }
            };
            let palette = self.palette_row(draw.palette, draw.shade, sheets);
            let position = (draw.position.x as i16 as u16 as u32)
                | ((draw.position.y as i16 as u16 as u32) << 16);
            let z_strip = draw.z_strip.unwrap_or_default();
            packed.extend([
                position,
                sprite,
                palette,
                draw.blitter as u32 | (u32::from(draw.z) << 16),
                u32::from(z_strip.first_width) | (u32::from(z_strip.initial_change as u8) << 8),
                z_strip.changes,
                pack_rgba(draw.outline),
            ]);
        }
        packed
    }

    /// Tile database index of each animation
    pub fn animations(&self) -> &[u16] {
        &self.animation_tiles
    }

    /// The buffers of the shader
    pub fn shared_buffers(&self) -> SharedBuffers {
        // The animation table follows the sprites
        let animations_offset = self.sprites.len() as u32;
        let mut frames_start = animations_offset + 2 * self.animations.len() as u32;
        let mut sprites = self.sprites.clone();
        for frames in &self.animations {
            sprites.extend([frames_start, frames.len() as u32]);
            frames_start += frames.len() as u32;
        }
        for frames in &self.animations {
            sprites.extend(frames);
        }
        SharedBuffers {
            atlas: self.atlas.clone(),
            palettes: self.palettes.clone(),
            sprites,
            animations_offset,
            animations: self.animation_tiles.clone(),
        }
    }

    /// Number of sprites and bytes of sprite data
    pub fn sprite_stats(&self) -> (usize, usize) {
        (self.sprites.len() / SPRITE_WORDS, self.atlas_len)
    }

    fn changed(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    fn push_atlas_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let shift = (self.atlas_len % 4) * 8;
            if shift == 0 {
                self.atlas.push(0);
            }
            if let Some(word) = self.atlas.last_mut() {
                *word |= u32::from(byte) << shift;
            }
            self.atlas_len += 1;
        }
    }

    /// Index of a sprite in the sprite table, `None` if it can't be drawn
    fn sprite(&mut self, key: SpriteKey, sheets: &SpriteSheets) -> Option<u32> {
        if let Some(slot) = self.sprite_slots.get(&key) {
            return *slot;
        }
        let slot = sheets
            .geometry(key)
            .zip(sheets.pixels(key))
            .map(|(geometry, pixels)| {
                let slot = (self.sprites.len() / SPRITE_WORDS) as u32;
                let offset = self.atlas_len as u32;
                self.push_atlas_bytes(pixels);
                self.sprites.extend([
                    offset,
                    geometry.size.x | (geometry.size.y << 16),
                    (geometry.offset.x as i16 as u16 as u32)
                        | ((geometry.offset.y as i16 as u16 as u32) << 16),
                ]);
                slot
            });
        self.sprite_slots.insert(key, slot);
        if slot.is_some() {
            self.changed();
        }
        slot
    }

    /// Id of the animation of a tile
    fn animation(&mut self, animation: u16, sheets: &SpriteSheets) -> u32 {
        if let Some(&slot) = self.animation_slots.get(&animation) {
            return slot;
        }
        let frames = sheets
            .tileset
            .tile(animation)
            .and_then(|tile| tile.animation.as_ref())
            .map(|animation| animation.frames.clone())
            .unwrap_or_default();
        // Frames that can't be drawn show the first sprite of the atlas, like a missing image.
        // `build_draw_list` only creates animations with at least one drawable frame.
        let frames = frames
            .into_iter()
            .map(|frame| self.sprite(SpriteKey::Tile(frame), sheets).unwrap_or(0))
            .collect::<Vec<_>>();
        let slot = self.animations.len() as u32;
        self.animations.push(frames);
        self.animation_tiles.push(animation);
        self.animation_slots.insert(animation, slot);
        self.changed();
        slot
    }

    /// Row of a palette at a shade level
    fn palette_row(&mut self, key: PaletteKey, shade: u8, sheets: &SpriteSheets) -> u32 {
        let first = match self.palette_rows.get(&key) {
            Some(&first) => first,
            None => {
                let first = (self.palettes.len() / 256) as u32;
                let light = match key {
                    PaletteKey::SoldierUnbiased(_) => [0; 3],
                    _ => self.light,
                };
                match sheets.palette(key) {
                    Some(palette) => {
                        for row in shaded_palettes(palette, light) {
                            self.palettes.extend(row);
                        }
                    }
                    // Not reachable for drawable sprites, use black
                    None => self
                        .palettes
                        .extend(std::iter::repeat_n(0xff00_0000, 256 * SHADE_LEVELS)),
                }
                self.palette_rows.insert(key, first);
                self.changed();
                first
            }
        };
        first + u32::from(shade.min(SHADE_LEVELS as u8 - 1))
    }
}

/// The draws and bins of an area, packed with a `SceneAtlas`
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackedDraws {
    /// `DRAW_WORDS` per draw
    pub draws: Vec<u32>,
    /// Bin offsets, then the draw indices of the bins
    pub bins: Vec<u32>,
    pub bins_origin: IVec2,
    pub bins_count: UVec2,
    /// Start of the draw indices in `bins`
    pub bin_indices_offset: u32,
}

impl PackedDraws {
    /// Packs the draws that overlap `area` (in draw list order) with bins covering `area`
    pub fn build(
        draws: &[Draw],
        area: IRect,
        atlas: &mut SceneAtlas,
        sheets: &SpriteSheets,
    ) -> Self {
        let bins = Bins::build(area, draws.iter().map(|draw| draw.bounds));
        let mut bin_buffer = bins.offsets;
        let bin_indices_offset = bin_buffer.len() as u32;
        bin_buffer.extend(&bins.indices);
        Self {
            draws: atlas.pack_draws(draws, sheets),
            bins: bin_buffer,
            bins_origin: bins.origin,
            bins_count: bins.count,
            bin_indices_offset,
        }
    }
}

/// The data of the replay shader for an area, as the material sees it
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuScene {
    /// `DRAW_WORDS` per draw
    pub draws: Vec<u32>,
    /// Bin offsets, then the draw indices of the bins
    pub bins: Vec<u32>,
    /// Palette indices of the sprites, 4 per word, little endian
    pub atlas: Vec<u32>,
    /// Rows of 256 colours: the shadow table, then the used palettes and shade levels
    pub palettes: Vec<u32>,
    /// `SPRITE_WORDS` per sprite, then per animation the start and number of its frames and
    /// finally the sprite indices of the frames
    pub sprites: Vec<u32>,
    /// Current frame of each animation
    pub animation_frames: Vec<u32>,
    /// Tile database index of each animation, to update `animation_frames`
    pub animations: Vec<u16>,
    pub params: SceneParams,
}

impl GpuScene {
    /// The scene of a draw list with its own atlas
    pub fn build(draws: &[Draw], bins: &Bins, sheets: &SpriteSheets, light: [u8; 3]) -> Self {
        let mut atlas = SceneAtlas::new(light);
        let packed = atlas.pack_draws(draws, sheets);
        let mut bin_buffer = bins.offsets.clone();
        let bin_indices_offset = bin_buffer.len() as u32;
        bin_buffer.extend(&bins.indices);
        let packed = PackedDraws {
            draws: packed,
            bins: bin_buffer,
            bins_origin: bins.origin,
            bins_count: bins.count,
            bin_indices_offset,
        };
        Self::from_parts(&atlas.shared_buffers(), &packed, sheets.tileset)
    }

    /// The scene of packed draws with the buffers of their atlas
    pub fn from_parts(shared: &SharedBuffers, packed: &PackedDraws, tileset: &Tileset) -> Self {
        GpuScene {
            draws: packed.draws.clone(),
            bins: packed.bins.clone(),
            atlas: shared.atlas.clone(),
            palettes: shared.palettes.clone(),
            sprites: shared.sprites.clone(),
            animation_frames: current_animation_frames(&shared.animations, tileset),
            animations: shared.animations.clone(),
            params: SceneParams {
                bins_origin: packed.bins_origin,
                bins_count: packed.bins_count,
                bin_indices_offset: packed.bin_indices_offset,
                animations_offset: shared.animations_offset,
            },
        }
    }
}

/// Current frame of each animation, the animations are tile ids
pub fn current_animation_frames(animations: &[u16], tileset: &Tileset) -> Vec<u32> {
    animations
        .iter()
        .map(|&animation| u32::from(tileset.current_frame_number(animation)))
        .collect()
}

/// Packs bytes into little endian words, the shader reads byte `i` from word `i / 4`
#[cfg(test)]
fn pack_bytes(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks(4)
        .map(|chunk| {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            u32::from_le_bytes(word)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_bytes_little_endian() {
        assert_eq!(pack_bytes(&[1, 2, 3, 4, 5]), vec![0x0403_0201, 5]);
        assert_eq!(pack_bytes(&[]), Vec::<u32>::new());
        // The atlas appends bytes the same way
        let mut atlas = SceneAtlas::new([0; 3]);
        atlas.push_atlas_bytes(&[1, 2, 3]);
        atlas.push_atlas_bytes(&[4, 5]);
        assert_eq!(atlas.atlas, pack_bytes(&[1, 2, 3, 4, 5]));
    }
}
