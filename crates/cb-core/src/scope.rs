//! The railgun's scope (openspec change `railgun-scope`; survey G24, owner, 2026-10-26: a magnified window in the
//! middle of the front screen, at 4 and 10 times; the pilot's own eyes never zoom). The rules the cockpit's scope
//! window is drawn by: its field of view for a magnification, how far the mouse turns the view while scoped, which
//! magnification a weapon offers. Pure; `cb-app::scope` renders the window from them.

use crate::weapons::WeaponSpec;

/// The scope window's vertical field of view, radians, for the pilot's view `view_fov_rad` at `zoom` times:
/// `2 atan(tan(view / 2) / zoom)`, so an object fills `zoom` times as much of the window as of the view.
pub fn fov_rad(view_fov_rad: f32, zoom: f32) -> f32 {
    2.0 * ((view_fov_rad * 0.5).tan() / zoom.max(1.0)).atan()
}

/// How far the mouse turns the view while scoped at `zoom`: one over the magnification, so the picture in the
/// window moves as far as it would unscoped. The player still turns the view; nothing turns it for them.
pub fn scoped_look(delta_deg: glam::Vec2, zoom: f32) -> glam::Vec2 {
    delta_deg / zoom.max(1.0)
}

/// The scope a mech offers: the magnifications of the selected station if it is scoped, else of the first scoped
/// weapon on board; empty when nothing on board has a scope (holding Scope then does nothing).
pub fn zooms(weapons: &[WeaponSpec], selected: usize) -> &[f32] {
    weapons
        .get(selected)
        .filter(|w| !w.scope_zoom.is_empty())
        .or_else(|| weapons.iter().find(|w| !w.scope_zoom.is_empty()))
        .map_or(&[], |w| w.scope_zoom.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec3};

    #[test]
    fn four_times_narrows_an_85_degree_view_to_25_point_8() {
        let at = |zoom: f32| fov_rad(85f32.to_radians(), zoom).to_degrees();
        assert!((at(4.0) - 25.8).abs() < 0.1, "{}", at(4.0));
        assert!((at(10.0) - 10.5).abs() < 0.1, "{}", at(10.0));
        let seated = fov_rad(95f32.to_radians(), 4.0).to_degrees();
        assert!((seated - 30.5).abs() < 0.1, "the seated 95 degree view: {seated}");
    }

    #[test]
    fn scoped_look_moves_a_quarter_as_far_at_4x() {
        assert_eq!(scoped_look(Vec2::new(8.0, -4.0), 4.0), Vec2::new(2.0, -1.0));
        assert_eq!(
            scoped_look(Vec2::new(20.0, 0.0), 10.0),
            Vec2::new(2.0, 0.0),
            "20 degrees becomes 2 at 10x"
        );
    }

    #[test]
    fn the_drop_mark_at_2_km_is_3_point_4_m_up_and_the_slug_lands_on_it() {
        let target = Vec3::new(0.0, 0.0, -2000.0);
        let mark = crate::weapons::lead(Vec3::ZERO, target, Vec3::ZERO, 2400.0, true);
        assert!((mark.y - 3.4).abs() < 0.05, "{mark}");
        // A slug fired at the mark, flown in fixed steps, comes down on the target.
        let mut pos = Vec3::ZERO;
        let mut vel = mark.normalize() * 2400.0;
        while pos.z > -2000.0 {
            pos += vel / 60.0;
            vel.y -= 9.81 / 60.0;
        }
        assert!(pos.y.abs() < 0.5, "{pos}");
    }

    #[test]
    fn only_a_scoped_weapon_offers_a_scope() {
        let mut w = crate::weapons::default_weapons();
        assert!(zooms(&w, 0).is_empty());
        w[1].scope_zoom = vec![4.0, 10.0];
        assert_eq!(zooms(&w, 0), &[4.0, 10.0], "on board, though not selected");
    }
}
