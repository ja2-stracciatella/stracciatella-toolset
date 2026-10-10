use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    text::Justify,
    window::PrimaryWindow,
};

use crate::{
    components::map::{game_to_world, JA2_TILE_HEIGHT, JA2_TILE_WIDTH},
    map::GridNo,
    render::world::{LevelNode, MapWorld},
    states::{DebugOverlay, UiFont},
    systems::{
        cursor::{draw_diamond, CursorInfo},
        pixel_view::PixelView,
        MapSystems,
    },
};

const FONT_SIZE: f32 = 20.;

/// A single line of the debug display in the top left corner
#[derive(Component, Clone, Copy, Debug)]
enum DebugLine {
    Fps,
    Zoom,
    MouseScreenPosition,
    MouseWorldPosition,
    MouseGamePosition,
    MouseTilePosition,
    MouseTile,
}

impl DebugLine {
    const ALL: [DebugLine; 7] = [
        DebugLine::Fps,
        DebugLine::Zoom,
        DebugLine::MouseScreenPosition,
        DebugLine::MouseWorldPosition,
        DebugLine::MouseGamePosition,
        DebugLine::MouseTilePosition,
        DebugLine::MouseTile,
    ];

    fn label(
        &self,
        cursor_info: &CursorInfo,
        fps: Option<f64>,
        zoom: f32,
        mouse_screen_position: Option<Vec2>,
    ) -> String {
        let position = |label: &str, position: Option<Vec2>| match position {
            Some(position) => format!("{}: {:.2} x {:.2}", label, position.x, position.y),
            None => format!("{}: None", label),
        };
        match self {
            DebugLine::Fps => match fps {
                Some(fps) => format!("Current FPS: {:.0}", fps),
                None => "Current FPS: None".to_owned(),
            },
            DebugLine::Zoom => format!("Zoom: {:.2}", zoom),
            DebugLine::MouseScreenPosition => {
                position("Mouse Screen Position", mouse_screen_position)
            }
            DebugLine::MouseWorldPosition => {
                position("Mouse World Position", cursor_info.world_position)
            }
            DebugLine::MouseGamePosition => position("Mouse Game Pixel", cursor_info.game_position),
            DebugLine::MouseTilePosition => {
                position("Mouse Tile Position", cursor_info.tile_position)
            }
            DebugLine::MouseTile => match cursor_info.tile {
                Some(tile) => format!(
                    "Mouse Tile: {} x {}, grid number {}",
                    tile.x,
                    tile.y,
                    GridNo::from(tile).index()
                ),
                None => "Mouse Tile: None".to_owned(),
            },
        }
    }
}

/// Lists the tile definitions of the tile below the cursor
#[derive(Component)]
struct TileInfoText;

pub struct TileDebugPlugin;

impl Plugin for TileDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(DebugOverlay::On), spawn_debug_ui)
            .add_systems(
                Update,
                (update_debug_lines, update_tile_info, draw_debug_cursors)
                    .in_set(MapSystems::Debug)
                    .run_if(in_state(DebugOverlay::On)),
            );
    }
}

fn spawn_debug_ui(mut commands: Commands, font: Res<UiFont>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(25.),
                top: Val::Px(25.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            DespawnOnExit(DebugOverlay::On),
        ))
        .with_children(|parent| {
            for line in DebugLine::ALL {
                parent.spawn((
                    Text::default(),
                    font.text_font(FONT_SIZE),
                    TextColor(Color::WHITE),
                    line,
                ));
            }
        });

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(35.),
            top: Val::Px(35.),
            width: Val::Px(400.),
            ..default()
        },
        Text::default(),
        font.text_font(FONT_SIZE),
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Right),
        TileInfoText,
        DespawnOnExit(DebugOverlay::On),
    ));
}

fn set_text(text: &mut Text, value: String) {
    if text.0 != value {
        text.0 = value;
    }
}

fn update_debug_lines(
    diagnostics: Res<DiagnosticsStore>,
    cursor_info: Res<CursorInfo>,
    pixel_view: Res<PixelView>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut lines: Query<(&DebugLine, &mut Text)>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed());
    let mouse_screen_position = window.cursor_position();
    for (line, mut text) in &mut lines {
        set_text(
            &mut text,
            line.label(&cursor_info, fps, pixel_view.zoom, mouse_screen_position),
        );
    }
}

/// Name of a tile: the image file and the sub image
fn node_name(node: &LevelNode) -> String {
    let image = &node.tile.image;
    let filler = if image.underflow_filler {
        " (filler)"
    } else {
        ""
    };
    format!("{}-{}{}", image.surface.filename, image.sub_image, filler)
}

fn update_tile_info(
    cursor_info: Res<CursorInfo>,
    world: Option<Res<MapWorld>>,
    mut tile_info_text: Single<&mut Text, With<TileInfoText>>,
) {
    let mut lines = vec![];
    if let (Some(tile), Some(world)) = (cursor_info.tile, world) {
        let tile = world.tile(tile);
        let layers = [
            ("Land", &tile.land),
            ("Objects", &tile.objects),
            ("Structs", &tile.structs),
            ("Shadows", &tile.shadows),
            ("Roofs", &tile.roofs),
            ("On Roof", &tile.on_roofs),
            ("Topmost", &tile.topmost),
        ];
        lines.push(format!("Height {}", tile.height));
        for (name, nodes) in layers {
            if nodes.is_empty() {
                continue;
            }
            lines.push(format!("{} Layer", name));
            lines.extend(nodes.iter().map(node_name));
        }
    }
    set_text(&mut tile_info_text, lines.join("\n"));
}

fn draw_debug_cursors(mut gizmos: Gizmos, cursor_info: Res<CursorInfo>) {
    let Some(tile) = cursor_info.tile else {
        return;
    };
    let size = IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT);
    let center = game_to_world(tile.tile_box() + size / 2);
    draw_diamond(&mut gizmos, center, size.as_vec2(), Srgba::GREEN);
}
