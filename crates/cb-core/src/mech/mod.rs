//! The mech (openspec change `mech-rig`): the rig contract, its data, its movement and its walk.

pub mod config;
pub mod gait;
pub mod locomotion;
pub mod rig;

pub use config::{BoxSpec, GaitConfig, LocomotionConfig, MechConfig, NodeSpec};
pub use gait::{LegPose, LegRest, MechPose, RigRest, pose, solve_leg};
pub use locomotion::{LocomotionMode, MechControls, MechState, forward};
