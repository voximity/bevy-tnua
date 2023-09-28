use std::ops::Add;

use bevy::prelude::*;

/// Allows disabling Tnua for a specific entity.
///
/// This can be used to let some other system  temporarily take control over a character.
///
/// This component is not mandatory - if omitted, Tnua will just assume it is enabled for that
/// entity.
#[derive(Component, Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum TnuaToggle {
    /// Do not update the sensors, and do not apply forces from the motor.
    ///
    /// The platformer controller system will also not run and won't update the motor and output
    /// components (like [`TnuaPlatformerAnimatingOutput`](crate::TnuaPlatformerAnimatingOutput)).
    /// They will retain their last value from before `TnuaToggle::Disabled` was set.
    Disabled,
    /// Update the sensors, but do not apply forces from the motor.
    ///
    /// The platformer controller system will still run and still update the motor and output
    /// components (like [`TnuaPlatformerAnimatingOutput`](crate::TnuaPlatformerAnimatingOutput)),
    /// only the system that applies the motor forces will be disabled.
    SenseOnly,
    #[default]
    /// The backend behaves normally - it updates the sensors and applies forces from the motor.
    Enabled,
}

/// Newtonian state of the rigid body.
///
/// Tnua takes the position and rotation of the rigid body from its `GlobalTransform`, but things
/// like velocity are dependent on the physics engine. The physics backend is responsible for
/// updating this component from the physics engine during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors).
#[derive(Component, Debug)]
pub struct TnuaRigidBodyTracker {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    /// Angular velocity as the rotation axis multiplied by the rotation speed in radians per
    /// second. Can be extracted from a quaternion using [`Quat::xyz`].
    pub angvel: Vec3,
    pub gravity: Vec3,
}

impl Default for TnuaRigidBodyTracker {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            angvel: Vec3::ZERO,
            gravity: Vec3::ZERO,
        }
    }
}

/// Distance from another collider in a certain direction, and information on that collider.
///
/// The physics backend is responsible for updating this component from the physics engine during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors), usually by casting a ray
/// or a shape in the `cast_direction`.
#[derive(Component, Debug)]
pub struct TnuaProximitySensor {
    /// The cast origin in the entity's coord system.
    pub cast_origin: Vec3,
    /// The direction in world coord system (unmodified by the entity's transform)
    pub cast_direction: Vec3,
    /// Tnua will update this field according to its need. The backend only needs to read it.
    pub cast_range: f32,
    pub output: Option<TnuaProximitySensorOutput>,

    /// Used to prevent collision with obstacles the character squeezed into sideways.
    ///
    /// This is used to prevent <https://github.com/idanarye/bevy-tnua/issues/14>. When casting,
    /// Tnua checks if the entity the ray(/shape)cast hits is also in contact with the owner
    /// collider. If so, Tnua compares the contact normal with the cast direction.
    ///
    /// For legitimate hits, these two directions should be opposite. If Tnua casts downwards and
    /// hits the actual floor, the normal of the contact with it should point upward. Opposite
    /// directions means the dot product is closer to `-1.0`.
    ///
    /// Illigitimage hits hits would have perpendicular directions - hitting a wall (sideways) when
    /// casting downwards - which should give a dot product closer to `0.0`.
    ///
    /// This field is compared to the dot product to determine if the hit is valid or not, and can
    /// usually be left at the default value of `-0.5`.
    ///
    /// Positive dot products should not happen (hitting the ceiling?), but it's trivial to
    /// consider them as invalid.
    pub intersection_match_prevention_cutoff: f32,
}

impl Default for TnuaProximitySensor {
    fn default() -> Self {
        Self {
            cast_origin: Vec3::ZERO,
            cast_direction: -Vec3::Y,
            cast_range: 0.0,
            output: None,
            intersection_match_prevention_cutoff: -0.5,
        }
    }
}

/// Information from [`TnuaProximitySensor`] that have detected another collider.
#[derive(Debug, Clone)]
pub struct TnuaProximitySensorOutput {
    /// The entity of the collider detected by the ray.
    pub entity: Entity,
    /// The distance to the collider from [`cast_origin`](TnuaProximitySensor::cast_origin) along the
    /// [`cast_direction`](TnuaProximitySensor::cast_direction).
    pub proximity: f32,
    /// The normal from the detected collider's surface where the ray hits.
    pub normal: Vec3,
    /// The velocity of the detected entity,
    pub entity_linvel: Vec3,
    /// The angular velocity of the detected entity, given as the rotation axis multiplied by the
    /// rotation speed in radians per second. Can be extracted from a quaternion using
    /// [`Quat::xyz`].
    pub entity_angvel: Vec3,
}

/// Represents a change to velocity (linear or angular)
#[derive(Debug)]
pub struct TnuaVelChange {
    // The part of the velocity change that gets multiplied by the frame duration.
    //
    // In Rapier, this is applied using `ExternalForce` so that the simulation will apply in
    // smoothly over time and won't be sensitive to frame rate.
    pub acceleration: Vec3,
    // The part of the velocity change that gets added to the velocity as-is.
    //
    // In Rapier, this is added directly to the `Velocity` component.
    pub boost: Vec3,
}

impl TnuaVelChange {
    pub const ZERO: Self = Self {
        acceleration: Vec3::ZERO,
        boost: Vec3::ZERO,
    };

    pub fn acceleration(acceleration: Vec3) -> Self {
        Self {
            acceleration,
            boost: Vec3::ZERO,
        }
    }

    pub fn boost(boost: Vec3) -> Self {
        Self {
            acceleration: Vec3::ZERO,
            boost,
        }
    }

    pub fn cancel_on_axis(&mut self, axis: Vec3) {
        self.acceleration = self.acceleration.reject_from(axis);
        self.boost = self.boost.reject_from(axis);
    }
}

impl Default for TnuaVelChange {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add<TnuaVelChange> for TnuaVelChange {
    type Output = TnuaVelChange;

    fn add(self, rhs: TnuaVelChange) -> Self::Output {
        Self::Output {
            acceleration: self.acceleration + rhs.acceleration,
            boost: self.boost + rhs.boost,
        }
    }
}

/// Instructions on how to move forces to the rigid body.
///
/// The physics backend is responsible for reading this component during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors) and apply the forces to the
/// rigid body.
///
/// This documentation uses the term "forces", but in fact these numbers ignore mass and are
/// applied directly to the velocity.
#[derive(Component, Default, Debug)]
pub struct TnuaMotor {
    /// How much velocity to add to the rigid body in the current frame.
    pub lin: TnuaVelChange,

    /// How much angular velocity to add to the rigid body in the current frame, given as the
    /// rotation axis multiplied by the rotation speed in radians per second. Can be extracted from
    /// a quaternion using [`Quat::xyz`].
    pub ang: TnuaVelChange,
}

/// An addon for [`TnuaProximitySensor`] that allows it to detect [`TnuaGhostPlatform`] colliders.
///
/// Tnua will register all the ghost platforms encountered by the proximity sensor inside this
/// component, so that other systems may pick one to override the [sensor
/// output](TnuaProximitySensor::output)
///
/// See <https://github.com/idanarye/bevy-tnua/wiki/Jump-fall-Through-Platforms>
///
/// See
/// [`TnuaSimpleFallThroughPlatformsHelper`](crate::control_helpers::TnuaSimpleFallThroughPlatformsHelper).
#[derive(Component, Default, Debug)]
pub struct TnuaGhostSensor(pub Vec<TnuaProximitySensorOutput>);

impl TnuaGhostSensor {
    pub fn iter(&self) -> impl Iterator<Item = &TnuaProximitySensorOutput> {
        self.0.iter()
    }
}

/// A marker for jump/fall-through platforms.
///
/// Ghost platforms must also have their solver groups (**not** collision groups) set to exclude
/// the character's collider. In order to sense them the player character's sensor must also use
/// [`TnuaGhostSensor`].
///
/// See <https://github.com/idanarye/bevy-tnua/wiki/Jump-fall-Through-Platforms>
///
/// See
/// [`TnuaSimpleFallThroughPlatformsHelper`](crate::control_helpers::TnuaSimpleFallThroughPlatformsHelper).
#[derive(Component, Default, Debug)]
pub struct TnuaGhostPlatform;
