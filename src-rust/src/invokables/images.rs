use crate::{invokables::Invokable, state::AppState};
use anyhow::{anyhow, Context, Result};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use stracciatella::file_formats::stci::{Stci, StciRgb888};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderImageFile {
    file: String,
    subimage: Option<usize>,
}

impl Invokable for RenderImageFile {
    type Output = Base64Image;

    fn name() -> &'static str {
        "render_image_file"
    }

    fn validate(&self) -> Result<()> {
        if self.file.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &AppState) -> Result<Self::Output> {
        let state = state.read();
        let mut f = state.open_file(&self.file).context("failed to read file")?;
        let stci = Stci::from_input(&mut f)?;
        let sub_image = self.subimage.unwrap_or(0);
        let size = match &stci {
            Stci::Indexed { sub_images, .. } => {
                let num_subimages = sub_images.len();
                sub_images
                    .get(sub_image)
                    .ok_or_else(move || {
                        anyhow!("indexed stci only contains {} subimages", num_subimages)
                    })
                    .map(|s| (u32::from(s.dimensions.0), u32::from(s.dimensions.1)))
            }
            Stci::Rgb { width, height, .. } => {
                if sub_image != 0 {
                    Err(anyhow!("rgb stci only contains 1 subimage"))
                } else {
                    Ok((u32::from(*width), u32::from(*height)))
                }
            }
        }?;
        let mut image = RgbaImage::new(size.0, size.1);

        match &stci {
            Stci::Indexed {
                sub_images,
                palette,
            } => {
                let sub_image = &sub_images[sub_image];
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadImageMetadata {
    file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubImageMetadata {
    width: u16,
    height: u16,
    offset_x: i16,
    offset_y: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFileMetadata {
    images: Vec<SubImageMetadata>,
}

impl Invokable for ReadImageMetadata {
    type Output = ImageFileMetadata;

    fn name() -> &'static str {
        "read_image_metadata"
    }

    fn invoke(&self, state: &AppState) -> Result<Self::Output> {
        let state = state.read();
        let mut f = state.open_file(&self.file).context("failed to read file")?;
        let stci = Stci::from_input(&mut f)?;
        let images = match stci {
            Stci::Indexed { sub_images, .. } => sub_images
                .iter()
                .map(|s| SubImageMetadata {
                    width: s.dimensions.0,
                    height: s.dimensions.1,
                    offset_x: s.offset.0,
                    offset_y: s.offset.1,
                })
                .collect(),
            Stci::Rgb { width, height, .. } => vec![SubImageMetadata {
                width,
                height,
                offset_x: 0,
                offset_y: 0,
            }],
        };

        Ok(ImageFileMetadata { images })
    }
}
