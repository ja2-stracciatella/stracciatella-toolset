//! The game's soldier definitions: teams, soldier classes, body types and their standing animations
//!
//! Each kind of definition is a table of structs with dense ids (the entry at index `n` has id
//! `n`). [`SoldierPlugin`] registers all of them as [`Registry`] resources when a map starts
//! loading. Definitions refer to each other, and are referred to, with `Arc`s from the registries.
//! The raw numbers of a map's soldier placements are checked against the registries when the map
//! is converted (`map::convert`), which also makes the random choices of this module (body type,
//! palette). Only validated placements, which carry the definitions, reach the renderer. The
//! palette files of soldiers are loaded here as well (`palette_files.rs`).

use std::{fmt::Debug, sync::Arc};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::states::AppState;

pub mod body_type;
pub mod class;
pub mod palette;
pub mod palette_files;
pub mod random;
pub mod standing_surface;
pub mod team;

use body_type::{soldier_body_types, SoldierBodyType};
use class::{SoldierClass, SOLDIER_CLASSES};
use standing_surface::{StandingSurface, STANDING_SURFACES};
use team::{Team, TEAMS};

/// The id of a definition, an index into its [`Registry`]
pub trait DefinitionId: Copy + Eq + Debug + Send + Sync + 'static {
    fn index(self) -> usize;
}

/// Defines an id newtype. The field is private to the module that invokes the macro, so ids can
/// only be created there, for the definitions and the named constants.
macro_rules! definition_id {
    ($(#[$meta:meta])* $name:ident($repr:ty)) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name($repr);

        impl $name {
            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl $crate::soldier::DefinitionId for $name {
            fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}
pub(crate) use definition_id;

/// A definition that is stored in a [`Registry`]
pub trait Definition: Send + Sync + 'static {
    type Id: DefinitionId;

    fn id(&self) -> Self::Id;
}

/// All definitions of a kind, indexed by their id
#[derive(Resource, Debug)]
pub struct Registry<T: Definition>(Vec<Arc<T>>);

impl<T: Definition> Registry<T> {
    fn new(definitions: impl IntoIterator<Item = T>) -> Self {
        Self(definitions.into_iter().map(Arc::new).collect())
    }

    /// The definition of an id. All ids have a definition, so this can't fail.
    pub fn get(&self, id: T::Id) -> &Arc<T> {
        &self.0[id.index()]
    }

    /// The definition of a raw number, `None` if there is none
    pub fn validate(&self, raw: impl TryInto<usize>) -> Option<Arc<T>> {
        let index = raw.try_into().ok()?;
        self.0.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<T>> {
        self.0.iter()
    }
}

pub type Teams = Registry<Team>;
pub type SoldierClasses = Registry<SoldierClass>;
pub type SoldierBodyTypes = Registry<SoldierBodyType>;
pub type StandingSurfaces = Registry<StandingSurface>;

/// All soldier definitions
#[derive(Clone, Copy)]
pub struct SoldierDefinitions<'a> {
    pub teams: &'a Teams,
    pub classes: &'a SoldierClasses,
    pub bodies: &'a SoldierBodyTypes,
    pub surfaces: &'a StandingSurfaces,
}

/// All soldier registries, built from the hardcoded tables
pub(crate) struct Registries {
    pub teams: Teams,
    pub classes: SoldierClasses,
    pub bodies: SoldierBodyTypes,
    pub surfaces: StandingSurfaces,
}

impl Registries {
    pub fn build() -> Self {
        let surfaces = StandingSurfaces::new(STANDING_SURFACES);
        Self {
            teams: Teams::new(TEAMS),
            classes: SoldierClasses::new(SOLDIER_CLASSES),
            bodies: SoldierBodyTypes::new(soldier_body_types(&surfaces)),
            surfaces,
        }
    }

    #[cfg(test)]
    pub fn definitions(&self) -> SoldierDefinitions<'_> {
        SoldierDefinitions {
            teams: &self.teams,
            classes: &self.classes,
            bodies: &self.bodies,
            surfaces: &self.surfaces,
        }
    }
}

/// The registered soldier definitions as a system parameter
#[derive(SystemParam)]
pub struct SoldierRegistries<'w> {
    teams: Res<'w, Teams>,
    classes: Res<'w, SoldierClasses>,
    bodies: Res<'w, SoldierBodyTypes>,
    surfaces: Res<'w, StandingSurfaces>,
}

impl SoldierRegistries<'_> {
    pub fn definitions(&self) -> SoldierDefinitions<'_> {
        SoldierDefinitions {
            teams: &self.teams,
            classes: &self.classes,
            bodies: &self.bodies,
            surfaces: &self.surfaces,
        }
    }
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoldierSystems {
    /// Inserts the registries when a map starts loading
    Register,
}

pub struct SoldierPlugin;

impl Plugin for SoldierPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<palette_files::PaletteReplacements>()
            .init_asset_loader::<palette_files::PaletteReplacementsLoader>()
            .init_asset::<palette_files::ColPalette>()
            .init_asset_loader::<palette_files::ColPaletteLoader>()
            .add_systems(
                OnEnter(AppState::Loading),
                register_soldier_definitions.in_set(SoldierSystems::Register),
            );
    }
}

fn register_soldier_definitions(mut commands: Commands) {
    let registries = Registries::build();
    commands.insert_resource(registries.teams);
    commands.insert_resource(registries.classes);
    commands.insert_resource(registries.bodies);
    commands.insert_resource(registries.surfaces);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_dense<T: Definition>(registry: &Registry<T>) {
        for (index, definition) in registry.iter().enumerate() {
            assert_eq!(definition.id().index(), index, "{:?}", definition.id());
        }
    }

    #[test]
    fn ids_are_dense() {
        let registries = Registries::build();
        assert_dense(&registries.teams);
        assert_dense(&registries.classes);
        assert_dense(&registries.bodies);
        assert_dense(&registries.surfaces);
    }

    #[test]
    fn validates_raw_ids() {
        let registries = Registries::build();
        assert_eq!(
            registries.bodies.validate(27i8).map(|body| body.id),
            Some(body_type::SoldierBodyTypeId::JEEP)
        );
        assert!(registries.bodies.validate(28i8).is_none());
        assert!(registries.bodies.validate(-1i8).is_none());
        assert!(registries.teams.validate(6i8).is_none());
        assert!(registries.classes.validate(9u8).is_none());
    }
}
