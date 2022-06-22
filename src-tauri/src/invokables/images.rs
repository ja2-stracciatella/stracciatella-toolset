use async_fs::OpenOptions;
use futures_lite::AsyncReadExt;
use image::RgbaImage;
use serde::Serialize;
use stracciatella::{
    file_formats::stci::{Stci, StciRgb888},
    unicode::Nfc,
    vfs::VfsLayer,
};

use crate::{
    error::{Error, Result},
    state,
};

#[derive(Debug)]
pub struct Base64Image {
    image: image::RgbaImage,
}

impl Base64Image {
    pub fn new(image: image::RgbaImage) -> Self {
        Self { image }
    }

    pub fn to_png_data(&self) -> core::result::Result<Vec<u8>, image::ImageError> {
        let mut png_data = vec![];
        let png_encoder = image::png::PngEncoder::new(&mut png_data);
        png_encoder.encode(
            self.image.as_raw(),
            self.image.width(),
            self.image.height(),
            image::ColorType::Rgba8,
        )?;
        Ok(png_data)
    }
}

impl Serialize for Base64Image {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = self.to_png_data().map_err(serde::ser::Error::custom)?;
        let base64_data = base64::encode(&data);
        let mime_type = "image/png";
        serializer.serialize_str(&format!("data:{};base64,{}", mime_type, base64_data))
    }
}

#[tauri::command]
pub async fn read_image_file(
    state: tauri::State<'_, state::TauriState>,
    file: String,
) -> Result<Base64Image> {
    if file.contains("..") {
        return Err(Error::new("file path cannot contain `..`".to_owned()));
    }

    let state = state.read().await;
    let selected_mod = super::mods::get_selected_mod(&state)?.clone();
    let path = super::mods::get_mod_data_path(&selected_mod.m, &file);

    let content: Vec<u8> = if path.exists() {
        let mut f = OpenOptions::new().open(&path).await?;
        let mut result = vec![];

        f.read_to_end(&mut result).await?;

        result
    } else {
        let mut f = selected_mod.vfs.open(&Nfc::caseless(&file))?;
        let mut result = vec![];

        f.read_to_end(&mut result)?;

        result
    };
    let stci = Stci::from_input(&mut content.as_slice())?;
    let size = match &stci {
        Stci::Indexed { sub_images, .. } => sub_images
            .get(0)
            .ok_or_else(|| {
                Error::new("indexed stci does not have at least one subimage".to_owned())
            })
            .map(|s| (u32::from(s.dimensions.0), u32::from(s.dimensions.1))),
        Stci::Rgb { width, height, .. } => Ok((u32::from(*width), u32::from(*height))),
    }?;
    let mut image = RgbaImage::new(size.0, size.1);

    match &stci {
        Stci::Indexed {
            sub_images,
            palette,
        } => {
            let sub_image = &sub_images[0];
            let width = usize::from(sub_image.dimensions.0);
            let height = usize::from(sub_image.dimensions.1);
            for y in 0..height {
                for x in 0..width {
                    let index = (y * width) + x;
                    let stci_pixel = palette.colors[usize::from(sub_image.data[index])];
                    let pixel = image.get_pixel_mut(x as u32, y as u32);

                    if sub_image.data[index] == 0 {
                        pixel[3] = 0;
                    } else {
                        pixel[0] = stci_pixel.0;
                        pixel[1] = stci_pixel.1;
                        pixel[2] = stci_pixel.2;
                        pixel[3] = 255;
                    }
                }
            }
        }
        Stci::Rgb {
            width,
            height,
            data,
        } => {
            let width = usize::from(*width);
            let height = usize::from(*height);
            for y in 0..height {
                for x in 0..width {
                    let index = (y * width) + x;
                    let stci_pixel = StciRgb888::from(data[index]);
                    let pixel = image.get_pixel_mut(x as u32, y as u32);

                    pixel[0] = stci_pixel.0;
                    pixel[1] = stci_pixel.1;
                    pixel[2] = stci_pixel.2;
                    pixel[3] = 255;
                }
            }
        }
    }

    Ok(Base64Image::new(image))
}
