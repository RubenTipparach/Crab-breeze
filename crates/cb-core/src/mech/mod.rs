//! The mech (openspec change `mech-rig`): the rig contract, its data, its movement and its walk.

pub mod config;
pub mod damage;
pub mod gait;
pub mod locomotion;
pub mod rig;

pub use config::{BoxSpec, GaitConfig, JetConfig, LocomotionConfig, MechConfig, NodeSpec, SkateConfig};
pub use damage::{Damage, Hit, PartKind, PartSpec};
pub use gait::{LegPose, LegRest, MechPose, RigRest, pose, solve_leg};
pub use locomotion::{Budget, LocomotionMode, MechControls, MechState, forward};
