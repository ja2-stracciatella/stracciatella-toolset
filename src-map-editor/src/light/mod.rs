//! Light templates (`.lht`), the shapes of the light sprites

use std::{collections::HashMap, io, sync::Arc};

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use byteorder::{ReadBytesExt, LE};

/// Registers the light template asset and its loader
pub struct LightPlugin;

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<LightTemplate>()
            .init_asset_loader::<LightTemplateLoader>();
    }
}

/// The radii of the light templates that the editor places (`L-R01.LHT` to `L-R08.LHT`)
pub const LIGHT_RADII: std::ops::RangeInclusive<u8> = 1..=8;

/// The loaded light templates by lower case file name: the ones of the map lights and the ones
/// the editor places, templates that could not be loaded are missing
#[derive(Resource, Clone, Debug, Default)]
pub struct LightTemplates(pub HashMap<String, Arc<LightTemplate>>);

impl LightTemplates {
    /// The template of a file name, in any case
    pub fn get(&self, name: &str) -> Option<&Arc<LightTemplate>> {
        self.0.get(&name.to_lowercase())
    }

    /// The radii of the editor whose templates are loaded
    pub fn radii(&self) -> Vec<u8> {
        LIGHT_RADII
            .filter(|&radius| self.get(&crate::map::light_template_name(radius)).is_some())
            .collect()
    }
}

/// Flags of the ray list of a light template
pub const LIGHT_BACKLIGHT: u16 = 0x4000;
pub const LIGHT_NEW_RAY: u16 = 0x8000;

/// A node of a light template, a tile relative to the light (`LIGHT_NODE`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightNode {
    pub dx: i16,
    pub dy: i16,
    pub light: u8,
}

/// A light template file (`.lht`)
#[derive(Asset, TypePath, Clone, Debug, Default, PartialEq)]
pub struct LightTemplate {
    pub lights: Vec<LightNode>,
    /// Rays through the nodes: node indices, with `LIGHT_NEW_RAY` marking the start of a ray and
    /// `LIGHT_BACKLIGHT` as a flag
    pub rays: Vec<u16>,
}

impl LightTemplate {
    /// Port of `LightLoad`
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut input = io::Cursor::new(bytes);
        let num_lights = input.read_u16::<LE>()?;
        let lights = (0..num_lights)
            .map(|_| {
                let dx = input.read_i16::<LE>()?;
                let dy = input.read_i16::<LE>()?;
                let _flags = input.read_u8()?;
                let light = input.read_u8()?;
                Ok(LightNode { dx, dy, light })
            })
            .collect::<io::Result<_>>()?;
        let num_rays = input.read_u16::<LE>()?;
        let rays = (0..num_rays)
            .map(|_| input.read_u16::<LE>())
            .collect::<io::Result<_>>()?;
        Ok(Self { lights, rays })
    }
}

#[derive(Default, TypePath)]
pub struct LightTemplateLoader;

impl AssetLoader for LightTemplateLoader {
    type Asset = LightTemplate;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<LightTemplate, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        LightTemplate::parse(&bytes)
    }

    fn extensions(&self) -> &[&str] {
        &["lht", "LHT"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_light_templates() {
        let mut bytes = vec![];
        bytes.extend(2u16.to_le_bytes());
        for (dx, dy, light) in [(8191i16, 8191i16, 0u8), (1, -1, 3)] {
            bytes.extend(dx.to_le_bytes());
            bytes.extend(dy.to_le_bytes());
            bytes.extend([0, light]);
        }
        bytes.extend(3u16.to_le_bytes());
        for ray in [LIGHT_NEW_RAY, 1, LIGHT_BACKLIGHT | 1] {
            bytes.extend(ray.to_le_bytes());
        }
        let template = LightTemplate::parse(&bytes).unwrap();
        assert_eq!(
            template.lights[1],
            LightNode {
                dx: 1,
                dy: -1,
                light: 3
            }
        );
        assert_eq!(template.rays, vec![LIGHT_NEW_RAY, 1, LIGHT_BACKLIGHT | 1]);
        assert!(LightTemplate::parse(&bytes[..5]).is_err());
    }
}
