// Replays the blits of the game for every texel of a texture, the world texture of the view or the
// radar map. It is drawn as a fullscreen triangle into a viewport of the texture
// (`render/replay_pass.rs`).
//
// Each pixel starts with the clear colour and z 0 and applies the draws that cover it in draw list
// order, with the depth rules of the game's blitters. See `render/draw_list.rs` for the blitters,
// `render/atlas.rs` for the buffer layouts and `render/reference.rs` for the same function on the
// CPU.

struct ReplayParams {
    // Game pixel of the top left texel of the texture
    origin: vec2<i32>,
    // Game pixel of the top left corner of the first bin
    bins_origin: vec2<i32>,
    bins_count: vec2<u32>,
    // Start of the draw indices in `bins`
    bin_indices_offset: u32,
    // Start of the animation table in `sprites`
    animations_offset: u32,
    // Packed RGBA
    clear_color: u32,
}

struct Draw {
    // x and y as INT16
    position: u32,
    // Sprite index, or the animation id with the highest bit set
    sprite: u32,
    palette_row: u32,
    // Blitter in the lowest byte, z in the upper 16 bits
    blitter_z: u32,
    // Width of the first z strip, then the initial change as INT8
    z_strip: u32,
    // 2 bit changes of the following strips
    z_changes: u32,
    // Packed RGBA colour of index 254 for the outline blitters
    outline_color: u32,
}

@group(0) @binding(0) var<uniform> params: ReplayParams;
@group(0) @binding(1) var<storage, read> draws: array<Draw>;
@group(0) @binding(2) var<storage, read> bins: array<u32>;
@group(0) @binding(3) var<storage, read> atlas: array<u32>;
@group(0) @binding(4) var<storage, read> palettes: array<u32>;
@group(0) @binding(5) var<storage, read> sprites: array<u32>;
@group(0) @binding(6) var<storage, read> animation_frames: array<u32>;

const BIN_SIZE: i32 = 32;
const ANIMATED_SPRITE: u32 = 0x80000000u;
// Z change between two z strips (`Z_SUBLAYERS * 10`)
const Z_STRIP_DELTA: i32 = 80;
const HALF_TILE: u32 = 20u;
const SHADOW_INDEX: u32 = 254u;

const TRANSPARENT: u32 = 0u;
const TRANS_Z: u32 = 1u;
const TRANS_Z_NB: u32 = 2u;
const TRANS_Z_INC: u32 = 3u;
const TRANS_Z_INC_SAME_Z_BURNS_THROUGH: u32 = 4u;
const SHADOW_Z: u32 = 5u;
const SHADOW: u32 = 6u;
const OUTLINE_Z: u32 = 7u;
const OUTLINE_Z_PIXELATE_OBSCURED: u32 = 8u;
const TRANS_SHADOW_Z_NB: u32 = 9u;
const TRANS_SHADOW_Z_NB_OBSCURED: u32 = 10u;

// Sign extends the lower 16 bits
fn low_i16(value: u32) -> i32 {
    return (bitcast<i32>(value) << 16u) >> 16u;
}

fn high_i16(value: u32) -> i32 {
    return bitcast<i32>(value) >> 16u;
}

fn atlas_byte(index: u32) -> u32 {
    return (atlas[index >> 2u] >> ((index & 3u) * 8u)) & 0xffu;
}

fn sprite_index(sprite: u32) -> u32 {
    if (sprite & ANIMATED_SPRITE) == 0u {
        return sprite;
    }
    let animation = sprite & ~ANIMATED_SPRITE;
    let table = params.animations_offset + 2u * animation;
    let first = sprites[table];
    let count = max(sprites[table + 1u], 1u);
    return sprites[first + animation_frames[animation] % count];
}

// A colour channel under a shadow (`ShadeTable`), the table is the first palette row
fn shadow(color: u32) -> u32 {
    let r = palettes[color & 0xffu];
    let g = palettes[(color >> 8u) & 0xffu];
    let b = palettes[(color >> 16u) & 0xffu];
    return r | (g << 8u) | (b << 16u) | 0xff000000u;
}

// Z of a column of a sprite with z strips, as a 16 bit value
fn strip_z(z: u32, z_strip: u32, z_changes: u32, column: u32) -> u32 {
    let first_width = z_strip & 0xffu;
    let initial = (bitcast<i32>(z_strip << 16u) >> 24u);
    var strip = 0u;
    if column >= first_width {
        strip = 1u + (column - first_width) / HALF_TILE;
    }
    var level = i32(z) + initial * Z_STRIP_DELTA;
    for (var i = 0u; i < min(strip, 16u); i++) {
        let change = (bitcast<i32>(z_changes << (30u - 2u * i)) >> 30u);
        level += change * Z_STRIP_DELTA;
    }
    return bitcast<u32>(level) & 0xffffu;
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        return value / 12.92;
    }
    return pow((value + 0.055) / 1.055, 2.4);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // The fragments are at the texel centers, one game pixel per texel
    let pixel = params.origin + vec2<i32>(floor(position.xy));
    // The checker pattern of the obscured blitters
    let checker = ((pixel.x ^ pixel.y) & 1) == 0;

    var color = params.clear_color;
    var z = 0u;

    let bin = pixel - params.bins_origin;
    if all(bin >= vec2<i32>(0)) {
        let bin_xy = vec2<u32>(bin / BIN_SIZE);
        if all(bin_xy < params.bins_count) {
            let bin_index = bin_xy.y * params.bins_count.x + bin_xy.x;
            let start = bins[bin_index];
            let end = bins[bin_index + 1u];
            for (var i = start; i < end; i++) {
                let draw = draws[bins[params.bin_indices_offset + i]];
                let sprite = sprite_index(draw.sprite) * 3u;
                let size = sprites[sprite + 1u];
                let offset = sprites[sprite + 2u];
                let top_left = vec2<i32>(low_i16(draw.position), high_i16(draw.position))
                    + vec2<i32>(low_i16(offset), high_i16(offset));
                let local = pixel - top_left;
                let width = size & 0xffffu;
                let height = size >> 16u;
                if any(local < vec2<i32>(0)) || u32(local.x) >= width || u32(local.y) >= height {
                    continue;
                }
                let index = atlas_byte(sprites[sprite] + u32(local.y) * width + u32(local.x));
                if index == 0u {
                    continue;
                }

                let blitter = draw.blitter_z & 0xffu;
                let draw_z = draw.blitter_z >> 16u;
                let draw_color = palettes[draw.palette_row * 256u + index];
                switch blitter {
                    case TRANSPARENT: {
                        color = draw_color;
                    }
                    case TRANS_Z: {
                        if z <= draw_z {
                            z = draw_z;
                            color = draw_color;
                        }
                    }
                    case TRANS_Z_NB: {
                        if z <= draw_z {
                            color = draw_color;
                        }
                    }
                    case TRANS_Z_INC, TRANS_Z_INC_SAME_Z_BURNS_THROUGH: {
                        let column_z = strip_z(draw_z, draw.z_strip, draw.z_changes, u32(local.x));
                        if z < column_z || (blitter == TRANS_Z_INC_SAME_Z_BURNS_THROUGH && z == column_z) {
                            z = column_z;
                            color = draw_color;
                        }
                    }
                    case SHADOW_Z: {
                        if z < draw_z {
                            z = draw_z;
                            color = shadow(color);
                        }
                    }
                    case SHADOW: {
                        color = shadow(color);
                    }
                    case OUTLINE_Z: {
                        if z <= draw_z {
                            if index == SHADOW_INDEX {
                                color = draw.outline_color;
                            } else {
                                z = draw_z;
                                color = draw_color;
                            }
                        }
                    }
                    case OUTLINE_Z_PIXELATE_OBSCURED: {
                        var visible = checker;
                        if z < draw_z {
                            z = draw_z;
                            visible = true;
                        }
                        if visible {
                            color = select(draw_color, draw.outline_color, index == SHADOW_INDEX);
                        }
                    }
                    case TRANS_SHADOW_Z_NB: {
                        if z < draw_z {
                            if index == SHADOW_INDEX {
                                color = shadow(color);
                            } else {
                                color = draw_color;
                            }
                        }
                    }
                    case TRANS_SHADOW_Z_NB_OBSCURED: {
                        if index == SHADOW_INDEX {
                            if z < draw_z {
                                color = shadow(color);
                            }
                        } else if z <= draw_z || checker {
                            color = draw_color;
                        }
                    }
                    default: {}
                }
            }
        }
    }

    let rgb = vec3<f32>(
        f32(color & 0xffu),
        f32((color >> 8u) & 0xffu),
        f32((color >> 16u) & 0xffu),
    ) / 255.0;
    return vec4<f32>(srgb_to_linear(rgb.r), srgb_to_linear(rgb.g), srgb_to_linear(rgb.b), 1.0);
}
