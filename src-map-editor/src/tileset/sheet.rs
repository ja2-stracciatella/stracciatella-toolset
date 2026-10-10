//! A loaded STCI image, as the renderer draws it

use std::io;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};

use crate::file_formats::{ja2_stci::SubImageInfo, stci::Stci};

/// Palette and pixels of an indexed STCI, like the game keeps them
#[derive(Clone, Debug)]
pub struct IndexedImages {
    /// RGB colors of the palette indices
    pub palette: Box<[[u8; 3]; 256]>,
    /// Palette indices of each sub image, row by row. Index 0 is transparent.
    pub pixels: Vec<Vec<u8>>,
}

/// Position and size of a sub image
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubImage {
    pub offset: (i16, i16),
    /// Width and height
    pub dimensions: (u16, u16),
}

/// A loaded STCI file
///
/// The renderer draws the palette indices itself, so they are not converted to a texture.
#[derive(Asset, TypePath, Debug)]
pub struct StciSheet {
    pub sub_images: Vec<SubImage>,
    /// Number of frames of each direction of an animation surface, 0 if the file doesn't say
    pub frames_per_direction: u8,
    /// `None` for RGB STCIs, which the renderer does not support
    pub indexed: Option<IndexedImages>,
    /// The file metadata of each sub image, only the input of the tile database
    pub(crate) database_info: Vec<SubImageInfo>,
}

impl StciSheet {
    /// Reads an STI file, RGB images have no `indexed` images
    pub fn parse(bytes: &[u8]) -> io::Result<StciSheet> {
        let stci = Stci::from_input(&mut io::Cursor::new(bytes))?;

        let database_info = SubImageInfo::from_stci(&stci);
        let sub_images = database_info
            .iter()
            .map(|info| SubImage {
                offset: info.offset,
                dimensions: info.dimensions,
            })
            .collect();
        let frames_per_direction = database_info
            .first()
            .and_then(|info| info.app_data.as_ref())
            .map_or(0, |app_data| app_data.number_of_frames);
        let indexed = match stci {
            Stci::Indexed {
                palette,
                sub_images,
            } => Some(IndexedImages {
                palette: Box::new(palette.colors.map(|color| [color.0, color.1, color.2])),
                pixels: sub_images
                    .into_iter()
                    .map(|sub_image| sub_image.data)
                    .collect(),
            }),
            Stci::Rgb { .. } => None,
        };

        Ok(StciSheet {
            sub_images,
            frames_per_direction,
            indexed,
            database_info,
        })
    }
}

#[derive(Default, TypePath)]
pub struct StciLoader;

impl AssetLoader for StciLoader {
    type Asset = StciSheet;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<StciSheet, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let sheet = StciSheet::parse(&bytes)?;
        if sheet.indexed.is_none() {
            warn!(
                "{} is an RGB image, only indexed images can be drawn",
                load_context.path()
            );
        }
        Ok(sheet)
    }

    fn extensions(&self) -> &[&str] {
        &["STI", "sti"]
    }
}
