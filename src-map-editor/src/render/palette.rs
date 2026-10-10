//! Shaded palettes and the shadow colour function, with 8 bits per colour channel
//!
//! Port of `CreateBiasedShadedPalettes` (`Lighting.cc`), `Create16BPPPaletteShaded` (`HImage.cc`)
//! and `BuildShadeTable` (`Shading.cc`). The game quantizes the results to the 16 bit frame
//! buffer format, here they stay 8 bits per channel.

/// Number of shade levels of a palette. Lower levels are brighter, 4 is the unchanged palette.
pub const SHADE_LEVELS: usize = 16;

/// The palette level that shows the original colours (`DEFAULT_SHADE_LEVEL`)
pub const DEFAULT_SHADE_LEVEL: u8 = 4;

/// Colour scales of the shade levels in 1/256 (`gusShadeLevels`, JA2 Gold version). Level 0 is
/// monochrome.
const SHADE_SCALES: [[u32; 3]; SHADE_LEVELS] = [
    [500, 500, 500],
    [450, 450, 450],
    [350, 350, 350],
    [300, 300, 300],
    [255, 255, 255],
    [231, 199, 199],
    [209, 185, 185],
    [187, 171, 171],
    [165, 157, 157],
    [143, 143, 143],
    [121, 121, 129],
    [99, 99, 115],
    [77, 77, 101],
    [36, 36, 244],
    [18, 18, 224],
    [48, 222, 48],
];

/// How much of the colour is left under a shadow (`guiShadePercent`)
const SHADOW_FACTOR: f32 = 0.48;

/// A colour as the shaders use it: red in the lowest byte, then green, blue and alpha
pub fn pack_rgba(rgb: [u8; 3]) -> u32 {
    u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 255])
}

/// The palette at one shade level, packed with [`pack_rgba`]
///
/// `light` is the map's light colour, which is added to the palette first (`AddSaturatePalette`).
pub fn shade_palette(palette: &[[u8; 3]; 256], light: [u8; 3], level: usize) -> [u32; 256] {
    palette.map(|color| {
        let biased: [u32; 3] =
            std::array::from_fn(|c| (u32::from(color[c]) + u32::from(light[c])).min(255));
        let scaled: [u32; 3] = if level == 0 {
            let [r, g, b] = biased;
            let luminance = (r * 299 + g * 587 + b * 114) / 1000;
            SHADE_SCALES[0].map(|scale| scale * luminance / 256)
        } else {
            std::array::from_fn(|c| SHADE_SCALES[level][c] * biased[c] / 256)
        };
        pack_rgba(scaled.map(|value| value.min(255) as u8))
    })
}

/// All shade levels of a palette, see [`shade_palette`]
pub fn shaded_palettes(palette: &[[u8; 3]; 256], light: [u8; 3]) -> Vec<[u32; 256]> {
    (0..SHADE_LEVELS)
        .map(|level| shade_palette(palette, light, level))
        .collect()
}

/// A colour channel under a shadow (`ShadeTable`)
pub fn shadow(channel: u8) -> u8 {
    // Same float computation and truncation as the game
    (f32::from(channel) * SHADOW_FACTOR) as u8
}

/// [`shadow`] of every channel value, for the shaders
pub fn shadow_table() -> [u32; 256] {
    std::array::from_fn(|channel| u32::from(shadow(channel as u8)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette_with(color: [u8; 3]) -> Box<[[u8; 3]; 256]> {
        let mut palette = Box::new([[0; 3]; 256]);
        palette[1] = color;
        palette
    }

    #[test]
    fn default_level_keeps_colours() {
        let palette = palette_with([10, 128, 255]);
        let shaded = shade_palette(&palette, [0; 3], usize::from(DEFAULT_SHADE_LEVEL));
        // 255 * c / 256 rounds down, like in the game
        assert_eq!(shaded[1], pack_rgba([9, 127, 254]));
        assert_eq!(shaded[0], pack_rgba([0, 0, 0]));
    }

    #[test]
    fn levels_scale_and_saturate() {
        let palette = palette_with([100, 200, 50]);
        // 450 * c / 256, saturated
        assert_eq!(
            shade_palette(&palette, [0; 3], 1)[1],
            pack_rgba([175, 255, 87])
        );
        // Night: 77, 77, 101
        assert_eq!(
            shade_palette(&palette, [0; 3], 12)[1],
            pack_rgba([30, 60, 19])
        );
    }

    #[test]
    fn level_zero_is_monochrome() {
        let palette = palette_with([100, 200, 50]);
        // (100 * 299 + 200 * 587 + 50 * 114) / 1000 = 153, 500 * 153 / 256 = 298
        assert_eq!(
            shade_palette(&palette, [0; 3], 0)[1],
            pack_rgba([255, 255, 255])
        );
        let palette = palette_with([20, 30, 40]);
        // lumin 28, 500 * 28 / 256 = 54
        assert_eq!(
            shade_palette(&palette, [0; 3], 0)[1],
            pack_rgba([54, 54, 54])
        );
    }

    #[test]
    fn light_colour_is_added_first() {
        let palette = palette_with([100, 250, 0]);
        let shaded = shade_palette(&palette, [10, 10, 20], usize::from(DEFAULT_SHADE_LEVEL));
        assert_eq!(shaded[1], pack_rgba([109, 254, 19]));
    }

    #[test]
    fn shadows() {
        assert_eq!(shadow(0), 0);
        assert_eq!(shadow(1), 0);
        assert_eq!(shadow(100), 48);
        // 255 * 0.48 = 122.4
        assert_eq!(shadow(255), 122);
        let table = shadow_table();
        assert_eq!(table[100], 48);
        assert_eq!(table.len(), 256);
    }
}
