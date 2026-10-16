//! CPU version of the replay shader (`shaders/ja2_replay.wesl`), for tests
//!
//! It reads the same packed buffers as the shader, so tests of it also test the packing.

use bevy::prelude::*;

use crate::render::{
    atlas::{GpuScene, ANIMATED_SPRITE, DRAW_WORDS, SPRITE_WORDS},
    bins::BIN_SIZE,
    draw_list::{Blitter, PackedZStrip},
};

const SHADOW_INDEX: u32 = 254;

fn low_i16(value: u32) -> i32 {
    i32::from(value as u16 as i16)
}

fn high_i16(value: u32) -> i32 {
    i32::from((value >> 16) as u16 as i16)
}

impl GpuScene {
    fn atlas_byte(&self, index: u32) -> u32 {
        (self.atlas[(index >> 2) as usize] >> ((index & 3) * 8)) & 0xff
    }

    fn sprite_index(&self, sprite: u32) -> u32 {
        if sprite & ANIMATED_SPRITE == 0 {
            return sprite;
        }
        let animation = sprite & !ANIMATED_SPRITE;
        let table = (self.params.animations_offset + 2 * animation) as usize;
        let first = self.sprites[table];
        let count = self.sprites[table + 1].max(1);
        self.sprites[(first + self.animation_frames[animation as usize] % count) as usize]
    }

    fn shadow(&self, color: u32) -> u32 {
        let channel = |shift: u32| self.palettes[((color >> shift) & 0xff) as usize];
        channel(0) | (channel(8) << 8) | (channel(16) << 16) | 0xff00_0000
    }

    /// Colour of a game pixel, packed RGBA
    pub fn replay_pixel(&self, pixel: IVec2, clear_color: u32) -> u32 {
        let checker = ((pixel.x ^ pixel.y) & 1) == 0;
        let mut color = clear_color;
        let mut z = 0u32;

        let bin = pixel - self.params.bins_origin;
        if bin.x < 0 || bin.y < 0 {
            return color;
        }
        let bin = (bin / BIN_SIZE).as_uvec2();
        if bin.x >= self.params.bins_count.x || bin.y >= self.params.bins_count.y {
            return color;
        }
        let bin_index = (bin.y * self.params.bins_count.x + bin.x) as usize;
        let (start, end) = (self.bins[bin_index], self.bins[bin_index + 1]);
        for i in start..end {
            let draw_index = self.bins[(self.params.bin_indices_offset + i) as usize] as usize;
            let draw = &self.draws[draw_index * DRAW_WORDS..(draw_index + 1) * DRAW_WORDS];
            let sprite = self.sprite_index(draw[1]) as usize * SPRITE_WORDS;
            let size = self.sprites[sprite + 1];
            let offset = self.sprites[sprite + 2];
            let top_left = IVec2::new(low_i16(draw[0]), high_i16(draw[0]))
                + IVec2::new(low_i16(offset), high_i16(offset));
            let local = pixel - top_left;
            let (width, height) = (size & 0xffff, size >> 16);
            if local.x < 0 || local.y < 0 || local.x as u32 >= width || local.y as u32 >= height {
                continue;
            }
            let index =
                self.atlas_byte(self.sprites[sprite] + local.y as u32 * width + local.x as u32);
            if index == 0 {
                continue;
            }

            let blitter = draw[3] & 0xff;
            let draw_z = draw[3] >> 16;
            let draw_color = self.palettes[(draw[2] * 256 + index) as usize];
            match blitter {
                b if b == Blitter::Transparent as u32 => color = draw_color,
                b if b == Blitter::TransZ as u32 => {
                    if z <= draw_z {
                        z = draw_z;
                        color = draw_color;
                    }
                }
                b if b == Blitter::TransZNB as u32 => {
                    if z <= draw_z {
                        color = draw_color;
                    }
                }
                b if b == Blitter::TransZInc as u32
                    || b == Blitter::TransZIncSameZBurnsThrough as u32 =>
                {
                    let z_strip = PackedZStrip {
                        first_width: (draw[4] & 0xff) as u8,
                        initial_change: (draw[4] >> 8) as u8 as i8,
                        changes: draw[5],
                    };
                    let column_z = u32::from(z_strip.column_z(draw_z as u16, local.x as u32));
                    let same_z = b == Blitter::TransZIncSameZBurnsThrough as u32;
                    if z < column_z || (same_z && z == column_z) {
                        z = column_z;
                        color = draw_color;
                    }
                }
                b if b == Blitter::ShadowZ as u32 => {
                    if z < draw_z {
                        z = draw_z;
                        color = self.shadow(color);
                    }
                }
                b if b == Blitter::Shadow as u32 => color = self.shadow(color),
                b if b == Blitter::OutlineZ as u32 => {
                    if z <= draw_z {
                        if index == SHADOW_INDEX {
                            color = draw[6];
                        } else {
                            z = draw_z;
                            color = draw_color;
                        }
                    }
                }
                b if b == Blitter::OutlineZPixelateObscured as u32 => {
                    let mut visible = checker;
                    if z < draw_z {
                        z = draw_z;
                        visible = true;
                    }
                    if visible {
                        color = if index == SHADOW_INDEX {
                            draw[6]
                        } else {
                            draw_color
                        };
                    }
                }
                b if b == Blitter::TransShadowZNB as u32 => {
                    if z < draw_z {
                        color = if index == SHADOW_INDEX {
                            self.shadow(color)
                        } else {
                            draw_color
                        };
                    }
                }
                b if b == Blitter::TransShadowZNBObscured as u32 => {
                    if index == SHADOW_INDEX {
                        if z < draw_z {
                            color = self.shadow(color);
                        }
                    } else if z <= draw_z || checker {
                        color = draw_color;
                    }
                }
                _ => {}
            }
        }
        color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        map::GridNo,
        render::{
            atlas::SpriteSheets,
            bins::Bins,
            draw_list::{Draw, DrawSprite, PaletteKey, SpriteKey},
            palette::{pack_rgba, shade_palette, shadow},
        },
        tileset::{
            tests::{test_tile_id, test_tileset, FIRSTTEXTURE},
            IndexedImages, StciSheet, SubImage,
        },
    };

    const CLEAR: u32 = 0xff00_0000;

    /// A sheet with three 2×1 sprites: indices [1, 2], [3, 0] and [254, 1]
    fn sheet() -> StciSheet {
        let mut palette = Box::new([[0; 3]; 256]);
        palette[1] = [100, 0, 0];
        palette[2] = [0, 100, 0];
        palette[3] = [0, 0, 200];
        let sub_image = SubImage {
            offset: (0, 0),
            dimensions: (2, 1),
        };
        StciSheet {
            sub_images: vec![sub_image.clone(), sub_image.clone(), sub_image],
            frames_per_direction: 0,
            indexed: Some(IndexedImages {
                palette,
                pixels: vec![vec![1, 2], vec![3, 0], vec![254, 1]],
            }),
            database_info: vec![],
        }
    }

    fn draw(tile: u16, position: IVec2, blitter: Blitter, z: u16, shade: u8) -> Draw {
        Draw {
            position,
            sprite: DrawSprite::Fixed(SpriteKey::Tile(tile)),
            palette: PaletteKey::Tile(test_tileset().tile(tile).unwrap().image.surface.id),
            shade,
            blitter,
            z,
            z_strip: None,
            outline: [0; 3],
            bounds: IRect::from_corners(position, position + IVec2::new(2, 1)),
            grid_no: GridNo::new(0).unwrap(),
        }
    }

    #[test]
    fn replays_the_blitters_in_order() {
        let tileset = test_tileset();
        let sheet = sheet();
        let mut surface_sheets = vec![None; tileset.surfaces().len()];
        let surface = tileset.surface_of_type(FIRSTTEXTURE).unwrap();
        surface_sheets[surface.id.index()] = Some(&sheet);
        let sheets = SpriteSheets {
            tileset: &tileset,
            surface_sheets,
            soldier_sheets: vec![],
            soldier_palettes: vec![],
        };
        let first = test_tile_id(&tileset, FIRSTTEXTURE, 1);
        let second = test_tile_id(&tileset, FIRSTTEXTURE, 2);
        let draws = [
            draw(first, IVec2::new(0, 0), Blitter::Transparent, 0, 4),
            draw(second, IVec2::new(1, 0), Blitter::TransZ, 10, 4),
            draw(second, IVec2::new(0, 0), Blitter::TransZ, 5, 4),
            // Only darkens the first pixel, the second one has a higher z
            draw(first, IVec2::new(0, 0), Blitter::ShadowZ, 7, 4),
            // Same z, drawn without writing z
            draw(second, IVec2::new(1, 0), Blitter::TransZNB, 10, 12),
            // Lower z, hidden
            draw(first, IVec2::new(1, 0), Blitter::TransZNB, 9, 4),
        ];
        let bins = Bins::build(IRect::new(-4, -4, 8, 8), draws.iter().map(|d| d.bounds));
        let scene = GpuScene::build(&draws, &bins, &sheets, [0; 3]);

        let normal = shade_palette(sheet.indexed.as_ref().unwrap().palette.as_ref(), [0; 3], 4);
        let night = shade_palette(sheet.indexed.as_ref().unwrap().palette.as_ref(), [0; 3], 12);
        let [r, g, b, _] = normal[3].to_le_bytes();
        let shadowed = pack_rgba([shadow(r), shadow(g), shadow(b)]);
        assert_eq!(scene.replay_pixel(IVec2::new(0, 0), CLEAR), shadowed);
        assert_eq!(scene.replay_pixel(IVec2::new(1, 0), CLEAR), night[3]);
        // The last draw is hidden only where something with a higher z is
        assert_eq!(scene.replay_pixel(IVec2::new(2, 0), CLEAR), normal[2]);
        // Outside of the draws
        assert_eq!(scene.replay_pixel(IVec2::new(0, 1), CLEAR), CLEAR);
        assert_eq!(scene.replay_pixel(IVec2::new(-100, 0), CLEAR), CLEAR);
        // Only the used sprites and palettes are packed
        assert_eq!(scene.params.animations_offset, 2 * 3);
        assert_eq!(scene.palettes.len(), 256 * 17);
    }

    #[test]
    fn outline_blitters_draw_index_254_in_the_outline_color() {
        let tileset = test_tileset();
        let sheet = sheet();
        let mut surface_sheets = vec![None; tileset.surfaces().len()];
        let surface = tileset.surface_of_type(FIRSTTEXTURE).unwrap();
        surface_sheets[surface.id.index()] = Some(&sheet);
        let sheets = SpriteSheets {
            tileset: &tileset,
            surface_sheets,
            soldier_sheets: vec![],
            soldier_palettes: vec![],
        };
        let first = test_tile_id(&tileset, FIRSTTEXTURE, 1);
        let outlined = test_tile_id(&tileset, FIRSTTEXTURE, 3);
        let outline = [10, 20, 30];
        let item = |position, blitter, z| Draw {
            outline,
            ..draw(outlined, position, blitter, z, 4)
        };
        let draws = [
            item(IVec2::new(0, 0), Blitter::OutlineZ, 10),
            // The outline does not write z, so this darkens it
            draw(first, IVec2::new(0, 0), Blitter::ShadowZ, 5, 4),
            // Obscured by a draw with a higher z, only the checker pattern is drawn
            draw(first, IVec2::new(-1, 1), Blitter::TransZ, 20, 4),
            item(IVec2::new(-1, 1), Blitter::OutlineZPixelateObscured, 10),
            // Not obscured, drawn outside of the checker pattern too
            item(IVec2::new(1, 2), Blitter::OutlineZPixelateObscured, 10),
        ];
        let bins = Bins::build(IRect::new(-4, -4, 8, 8), draws.iter().map(|d| d.bounds));
        let scene = GpuScene::build(&draws, &bins, &sheets, [0; 3]);

        let normal = shade_palette(sheet.indexed.as_ref().unwrap().palette.as_ref(), [0; 3], 4);
        let shadowed = pack_rgba(outline.map(shadow));
        assert_eq!(scene.replay_pixel(IVec2::new(0, 0), CLEAR), shadowed);
        assert_eq!(scene.replay_pixel(IVec2::new(1, 0), CLEAR), normal[1]);
        let outline_color = pack_rgba(outline);
        assert_eq!(scene.replay_pixel(IVec2::new(-1, 1), CLEAR), outline_color);
        assert_eq!(scene.replay_pixel(IVec2::new(0, 1), CLEAR), normal[2]);
        assert_eq!(scene.replay_pixel(IVec2::new(1, 2), CLEAR), outline_color);
        assert_eq!(scene.replay_pixel(IVec2::new(2, 2), CLEAR), normal[1]);
    }
}
