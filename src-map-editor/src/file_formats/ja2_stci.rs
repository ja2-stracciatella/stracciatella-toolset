use crate::file_formats::stci::Stci;

use super::stci::StciAppData;

#[derive(Clone, Debug, PartialEq)]
pub struct SubImageInfo {
    pub offset: (i16, i16),
    /// Width and height
    pub dimensions: (u16, u16),
    /// Set for all sub images or for none, see `Stci::from_input`
    pub app_data: Option<StciAppData>,
}

impl SubImageInfo {
    /// Metadata of each sub image of an STCI, RGB STCIs have a single sub image
    pub fn from_stci(stci: &Stci) -> Vec<SubImageInfo> {
        match stci {
            Stci::Indexed { sub_images, .. } => sub_images
                .iter()
                .map(|sub_image| SubImageInfo {
                    offset: sub_image.offset,
                    dimensions: sub_image.dimensions,
                    app_data: sub_image.app_data.clone(),
                })
                .collect(),
            Stci::Rgb { width, height, .. } => vec![SubImageInfo {
                offset: (0, 0),
                dimensions: (*width, *height),
                app_data: None,
            }],
        }
    }
}
