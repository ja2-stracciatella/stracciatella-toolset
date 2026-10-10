//! The palette files of soldiers: the palette replacements (`ja2pal.dat`) and the palettes of
//! body types that don't use the palette of their animation (`.col`)

use std::io::{self, Read};

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use byteorder::{ReadBytesExt, LE};

use crate::file_formats::StracciatellaReadExt;

/// The palette replacements of soldiers (`LoadPaletteData`)
pub const SOLDIER_PALETTE_FILE: &str = "binarydata/ja2pal.dat";

/// A replacement of [`PaletteReplacements`], the index into its replacements
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaletteReplacementId(u16);

/// Named colour ranges that replace parts of a soldier palette (`ja2pal.dat`)
#[derive(Asset, TypePath, Clone, Debug, Default, PartialEq)]
pub struct PaletteReplacements {
    /// First and last palette index of each range
    ranges: Vec<(u8, u8)>,
    /// Range, name and colours
    replacements: Vec<(u8, String, Vec<[u8; 3]>)>,
}

impl PaletteReplacements {
    /// Port of `LoadPaletteData`
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut input = io::Cursor::new(bytes);
        let num_ranges = input.read_u32::<LE>()?;
        input.read_unused(num_ranges as usize)?;
        let ranges = (0..num_ranges)
            .map(|_| Ok((input.read_u8()?, input.read_u8()?)))
            .collect::<io::Result<_>>()?;
        let num_replacements = input.read_u32::<LE>()?;
        let replacements = (0..num_replacements)
            .map(|_| {
                let range = input.read_u8()?;
                let mut id = [0; 30];
                input.read_exact(&mut id)?;
                let end = id.iter().position(|&b| b == 0).unwrap_or(id.len());
                let id = String::from_utf8_lossy(&id[..end]).into_owned();
                let size = input.read_u8()?;
                let colors = (0..size)
                    .map(|_| Ok([input.read_u8()?, input.read_u8()?, input.read_u8()?]))
                    .collect::<io::Result<_>>()?;
                Ok((range, id, colors))
            })
            .collect::<io::Result<_>>()?;
        Ok(Self {
            ranges,
            replacements,
        })
    }

    /// The replacement with the given name
    pub fn find(&self, name: &str) -> Option<PaletteReplacementId> {
        let index = self.replacements.iter().position(|(_, n, _)| n == name)?;
        Some(PaletteReplacementId(index as u16))
    }

    /// Names of the replacements of a range, in their order (the editor's colour sets,
    /// `iEditColorStart`)
    pub fn names_in_range(&self, range: u8) -> impl Iterator<Item = &str> {
        self.replacements
            .iter()
            .filter(move |(r, _, _)| *r == range)
            .map(|(_, name, _)| name.as_str())
    }

    /// Name of a replacement
    pub fn name(&self, id: PaletteReplacementId) -> Option<&str> {
        let (_, name, _) = self.replacements.get(usize::from(id.0))?;
        Some(name)
    }

    /// Replacements of the given ranges with the given names and no colours
    #[cfg(test)]
    pub(crate) fn with_ranges(names: &[(u8, &str)]) -> Self {
        Self {
            ranges: vec![],
            replacements: names
                .iter()
                .map(|&(range, name)| (range, name.to_owned(), vec![]))
                .collect(),
        }
    }

    /// Replacements with the given names: those ending in `HEAD`, `PANTS` and `SKIN` replace
    /// palette indices 10 and 11, 12 and 13, 14 and 15, the others 16 and 17. Each has its own
    /// colours.
    #[cfg(test)]
    pub(crate) fn coloured(names: &[&str]) -> Self {
        Self {
            ranges: vec![(10, 11), (12, 13), (14, 15), (16, 17)],
            replacements: names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let range = ["HEAD", "PANTS", "SKIN"]
                        .iter()
                        .position(|suffix| name.ends_with(suffix))
                        .unwrap_or(3) as u8;
                    let color = [index as u8, 100, 200];
                    (range, (*name).to_owned(), vec![color, color])
                })
                .collect(),
        }
    }

    /// Replacements with the given names and no colours
    #[cfg(test)]
    pub(crate) fn with_names(names: &[&str]) -> Self {
        Self {
            ranges: vec![],
            replacements: names
                .iter()
                .map(|name| (0, (*name).to_owned(), vec![]))
                .collect(),
        }
    }

    /// Replaces the range of a replacement (`SetPaletteReplacement`)
    pub fn apply(&self, palette: &mut [[u8; 3]; 256], id: PaletteReplacementId) {
        let Some((range, _, colors)) = self.replacements.get(usize::from(id.0)) else {
            return;
        };
        let Some(&(start, end)) = self.ranges.get(usize::from(*range)) else {
            return;
        };
        for index in start..=end {
            if let Some(&color) = colors.get(usize::from(index - start)) {
                palette[usize::from(index)] = color;
            }
        }
    }
}

#[derive(Default, TypePath)]
pub struct PaletteReplacementsLoader;

impl AssetLoader for PaletteReplacementsLoader {
    type Asset = PaletteReplacements;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<PaletteReplacements, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        PaletteReplacements::parse(&bytes)
    }

    fn extensions(&self) -> &[&str] {
        &["dat"]
    }
}

/// A palette file (`.col`, `CreateSGPPaletteFromCOLFile`)
#[derive(Asset, TypePath, Clone, Debug, PartialEq)]
pub struct ColPalette(pub Box<[[u8; 3]; 256]>);

impl ColPalette {
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut input = io::Cursor::new(bytes);
        input.read_unused(8)?;
        let mut palette = Box::new([[0; 3]; 256]);
        for color in palette.iter_mut() {
            input.read_exact(color)?;
        }
        Ok(Self(palette))
    }
}

#[derive(Default, TypePath)]
pub struct ColPaletteLoader;

impl AssetLoader for ColPaletteLoader {
    type Asset = ColPalette;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<ColPalette, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        ColPalette::parse(&bytes)
    }

    fn extensions(&self) -> &[&str] {
        &["col", "COL"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_palette_ranges() {
        let mut bytes = vec![];
        bytes.extend(2u32.to_le_bytes());
        bytes.extend([1, 1]);
        bytes.extend([10, 11, 20, 20]);
        bytes.extend(2u32.to_le_bytes());
        for (range, id, colors) in [(0u8, "REDVEST", 2u8), (1, "PINKSKIN", 1)] {
            bytes.push(range);
            let mut name = [0u8; 30];
            name[..id.len()].copy_from_slice(id.as_bytes());
            bytes.extend(name);
            bytes.push(colors);
            for c in 0..colors {
                bytes.extend([c, 100, 200]);
            }
        }
        let replacements = PaletteReplacements::parse(&bytes).unwrap();
        let mut palette = [[7; 3]; 256];
        replacements.apply(&mut palette, replacements.find("REDVEST").unwrap());
        assert_eq!(replacements.find("UNKNOWN"), None);
        assert_eq!(palette[9], [7; 3]);
        assert_eq!(palette[10], [0, 100, 200]);
        assert_eq!(palette[11], [1, 100, 200]);
        assert_eq!(palette[20], [7; 3]);
        replacements.apply(&mut palette, replacements.find("PINKSKIN").unwrap());
        assert_eq!(palette[20], [0, 100, 200]);
    }
}
