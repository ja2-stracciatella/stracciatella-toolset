use bevy::prelude::*;

pub mod loading;
pub mod map;
pub mod tile_debug;

#[derive(States, Default, Clone, PartialEq, Eq, Hash, Debug)]
pub enum AppState {
    /// No map has been requested yet
    #[default]
    Idle,
    Loading,
    Map,
}

#[derive(SubStates, Default, Clone, PartialEq, Eq, Hash, Debug)]
#[source(AppState = AppState::Loading)]
pub enum LoadingPhase {
    #[default]
    MapAndTilesets,
    TileSheets,
    Failed,
}

/// Whether the tile debug overlay is shown on top of the map
#[derive(SubStates, Default, Clone, PartialEq, Eq, Hash, Debug)]
#[source(AppState = AppState::Map)]
pub enum DebugOverlay {
    #[default]
    Off,
    On,
}

/// Font used for all UI text
#[derive(Resource, Clone, Debug)]
pub struct UiFont(pub Handle<Font>);

impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        // Embedded, so there is no need to serve an assets directory
        let font = Font::from_bytes(
            include_bytes!("../../assets/fonts/SourceSansPro-Regular.ttf").to_vec(),
        );
        UiFont(world.resource_mut::<Assets<Font>>().add(font))
    }
}

impl UiFont {
    pub fn text_font(&self, size: f32) -> TextFont {
        TextFont::from(self.0.clone()).with_font_size(size)
    }
}
