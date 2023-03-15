//! # Tnua - A Character Controller for bevy_rapier.
//!
//! Tnua ("motion" in Hebrew) is a floating character controller, which means that instead of
//! constantly touching the ground the character floats above it, which makes many aspects of the
//! motion control simpler.
//!
//! Tnua uses [Rapier](https://rapier.rs/), and supports both the 2D and 3D versions of it:
//!
//! * For 2D, enable `features = ["rapier_2d"]` and use [`TnuaRapier2dPlugin`].
//! * For 3D, enable `features = ["rapier_3d"]` and use [`TnuaRapier3dPlugin`].
//!
//! In addition to the physics backend plugin, the [`TnuaPlatformerPlugin`] should also be added.
//!
//! A Tnua controlled character must have a dynamic rigid body, a `Velocity` component, and
//! everything from [`TnuaPlatformerBundle`]:
//! ```no_run
//! # use bevy::prelude::*;
//! # // Not importing from Rapier because there are two versions and the default features does not
//! # // enable either:
//! # type Velocity = ();
//! # #[derive(Component)]
//! # enum RigidBody { Dynamic }
//! # use bevy_tnua::{TnuaPlatformerBundle, TnuaPlatformerConfig, TnuaFreeFallBehavior};
//! # let mut commands: Commands = panic!();
//! # let mut cmd = commands.spawn_empty();
//! cmd.insert(RigidBody::Dynamic);
//! cmd.insert(Velocity::default());
//! cmd.insert(TnuaPlatformerBundle::new_with_config(
//!     TnuaPlatformerConfig {
//!         full_speed: 20.0,
//!         full_jump_height: 4.0,
//!         up: Vec3::Y,
//!         forward: -Vec3::Z,
//!         float_height: 2.0,
//!         cling_distance: 1.0,
//!         spring_strengh: 400.0,
//!         spring_dampening: 60.0,
//!         acceleration: 60.0,
//!         air_acceleration: 20.0,
//!         coyote_time: 0.15,
//!         jump_start_extra_gravity: 30.0,
//!         jump_fall_extra_gravity: 20.0,
//!         jump_shorten_extra_gravity: 40.0,
//!         free_fall_behavior: TnuaFreeFallBehavior::LikeJumpShorten,
//!         tilt_offset_angvel: 10.0,
//!         tilt_offset_angacl: 1000.0,
//!         turning_angvel: 10.0,
//!     },
//! ));
//! ```
//! Typically though it'd also include a `Collider`. Tnua will apply forces to keep the character
//! upright, but `LockedAxes` can also be used to prevent tilting entirely (without it the tilting
//! will be visible)
//!
//! To control the character, update the [`TnuaPlatformerControls`] in a system:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_tnua::{TnuaPlatformerControls};
//! fn player_control_system(mut query: Query<&mut TnuaPlatformerControls>) {
//!     for mut controls in query.iter_mut() {
//!         *controls = TnuaPlatformerControls {
//!             desired_velocity: Vec3::X, // always go right for some reason
//!             desired_forward: -Vec3::X, // face backwards from walking direction
//!             jump: None, // no jumping
//!         };
//!     }
//! }
//! ```
//! Tnua does not write to [`TnuaPlatformerControls`] - only reads from it - so it should be updated
//! every frame.
//!
//! If the [`TnuaPlatformerAnimatingOutput`] component is added to the entity, Tnua will keep it
//! updated with data that can be used to decide which animation to play.
//! a useful helper for that.
mod animating_helper;
#[cfg(feature = "rapier_2d")]
mod backend_rapier2d;
#[cfg(feature = "rapier_3d")]
mod backend_rapier3d;
mod platformer;
pub use animating_helper::{TnuaAnimatingState, TnuaAnimatingStateDirective};

#[cfg(feature = "rapier_2d")]
pub use backend_rapier2d::*;
#[cfg(feature = "rapier_3d")]
pub use backend_rapier3d::*;
pub use platformer::*;

mod data_for_backends;
pub use data_for_backends::*;

use bevy::prelude::*;

#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum TnuaSystemSet {
    Sensors,
    Logic,
    Motors,
}
