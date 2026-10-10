//! Reduces the colours of an image to a palette
//!
//! Port of the octree quantizer of the game (`Quantize.cc`), which the map utility uses for the
//! radar maps. Unlike the game, index 0 of the palette is left unused: it is transparent in
//! indexed STIs, so the colours start at index 1.

/// Bits of each channel that the octree looks at (`COLOUR_BITS`)
const COLOUR_BITS: usize = 6;
/// Most colours the tree keeps (`MAX_COLOURS`)
const MAX_COLOURS: usize = 255;

#[derive(Clone, Debug, Default)]
struct Node {
    leaf: bool,
    pixels: u32,
    sums: [u32; 3],
    children: [Option<usize>; 8],
}

#[derive(Default)]
struct Octree {
    /// All nodes ever created, reduced children stay behind unused
    nodes: Vec<Node>,
    leaves: usize,
    /// The nodes that can be reduced per level, the most recent last (`g_reducible_nodes`)
    reducible: [Vec<usize>; COLOUR_BITS],
}

impl Octree {
    /// `CreateNode`
    fn create_node(&mut self, level: usize) -> usize {
        let index = self.nodes.len();
        let leaf = level == COLOUR_BITS;
        self.nodes.push(Node {
            leaf,
            ..Default::default()
        });
        if leaf {
            self.leaves += 1;
        } else {
            self.reducible[level].push(index);
        }
        index
    }

    /// `AddColor`, starting at the root
    fn add_color(&mut self, color: [u8; 3]) {
        if self.nodes.is_empty() {
            self.create_node(0);
        }
        let mut node = 0;
        let mut level = 0;
        loop {
            if self.nodes[node].leaf {
                let node = &mut self.nodes[node];
                node.pixels += 1;
                for (sum, channel) in node.sums.iter_mut().zip(color) {
                    *sum += u32::from(channel);
                }
                return;
            }
            let shift = 7 - level;
            let bit = |channel: u8| usize::from(channel >> shift & 1);
            let child = bit(color[0]) << 2 | bit(color[1]) << 1 | bit(color[2]);
            node = match self.nodes[node].children[child] {
                Some(index) => index,
                None => {
                    let index = self.create_node(level + 1);
                    self.nodes[node].children[child] = Some(index);
                    index
                }
            };
            level += 1;
        }
    }

    /// Merges the children of the most recent node of the deepest level into it (`ReduceTree`)
    fn reduce(&mut self) {
        let Some(level) = (0..COLOUR_BITS)
            .rev()
            .find(|&level| !self.reducible[level].is_empty())
        else {
            return;
        };
        let Some(index) = self.reducible[level].pop() else {
            return;
        };
        let mut merged = Node {
            leaf: true,
            pixels: self.nodes[index].pixels,
            ..Default::default()
        };
        let mut children = 0;
        for child in self.nodes[index].children.iter().flatten() {
            let child = &self.nodes[*child];
            merged.pixels += child.pixels;
            for (sum, child_sum) in merged.sums.iter_mut().zip(child.sums) {
                *sum += child_sum;
            }
            children += 1;
        }
        self.nodes[index] = merged;
        self.leaves = self.leaves + 1 - children;
    }

    /// The average colours of the leaves, depth first (`GetPaletteColors`)
    fn colors(&self, node: usize, out: &mut Vec<[u8; 3]>) {
        let node = &self.nodes[node];
        if node.leaf {
            out.push(node.sums.map(|sum| (sum / node.pixels.max(1)) as u8));
        } else {
            for child in node.children.iter().flatten() {
                self.colors(*child, out);
            }
        }
    }
}

/// Reduces `pixels` to at most 255 colours, returns the palette and the palette index of each
/// pixel. Index 0 of the palette is black and not used by any pixel.
pub fn quantize(pixels: &[[u8; 3]]) -> (Box<[[u8; 3]; 256]>, Vec<u8>) {
    let mut tree = Octree::default();
    for &pixel in pixels {
        tree.add_color(pixel);
        while tree.leaves > MAX_COLOURS {
            tree.reduce();
        }
    }
    let mut colors = Vec::with_capacity(MAX_COLOURS);
    if !tree.nodes.is_empty() {
        tree.colors(0, &mut colors);
    }

    let mut palette = Box::new([[0; 3]; 256]);
    palette[1..=colors.len()].copy_from_slice(&colors);
    // The closest colour, the first one of equally close ones (`MapPalette`)
    let indices = pixels
        .iter()
        .map(|pixel| {
            let distance = |color: &[u8; 3]| -> u32 {
                color
                    .iter()
                    .zip(pixel)
                    .map(|(&a, &b)| (i32::from(a) - i32::from(b)).pow(2) as u32)
                    .sum()
            };
            let (best, _) = colors
                .iter()
                .enumerate()
                .fold((0, u32::MAX), |(best, lowest), (index, color)| {
                    let distance = distance(color);
                    if distance < lowest {
                        (index, distance)
                    } else {
                        (best, lowest)
                    }
                });
            (best + 1) as u8
        })
        .collect();
    (palette, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_colours_are_kept() {
        let pixels = [[255, 0, 0], [0, 255, 0], [255, 0, 0], [0, 0, 252], [8, 12, 16]];
        let (palette, indices) = quantize(&pixels);
        assert_eq!(palette[0], [0, 0, 0]);
        for (pixel, index) in pixels.iter().zip(&indices) {
            assert_ne!(*index, 0);
            assert_eq!(palette[usize::from(*index)], *pixel);
        }
        assert_eq!(indices[0], indices[2]);
    }

    #[test]
    fn colours_of_a_leaf_are_averaged() {
        // The same in the 6 bits that the tree looks at
        let (palette, indices) = quantize(&[[100, 100, 100], [102, 102, 102]]);
        assert_eq!(indices, vec![1, 1]);
        assert_eq!(palette[1], [101, 101, 101]);
    }

    #[test]
    fn many_colours_are_reduced() {
        let pixels: Vec<[u8; 3]> = (0..88 * 44)
            .map(|i| [(i * 7 % 256) as u8, (i * 13 % 256) as u8, (i / 16) as u8])
            .collect();
        let (palette, indices) = quantize(&pixels);
        assert_eq!(palette[0], [0, 0, 0]);
        assert!(indices.iter().all(|&index| index != 0));
        let used = indices.iter().copied().max().unwrap();
        assert!(used >= 200, "only {} colours used", used);
        // Every pixel is close to its colour
        for (pixel, index) in pixels.iter().zip(&indices) {
            let color = palette[usize::from(*index)];
            let distance: i32 = color
                .iter()
                .zip(pixel)
                .map(|(&a, &b)| (i32::from(a) - i32::from(b)).abs())
                .max()
                .unwrap();
            assert!(distance < 64, "{:?} became {:?}", pixel, color);
        }
        // And it is deterministic
        assert_eq!(quantize(&pixels), (palette, indices));
    }

    #[test]
    fn no_pixels() {
        let (palette, indices) = quantize(&[]);
        assert_eq!(*palette, [[0; 3]; 256]);
        assert!(indices.is_empty());
    }
}
