//! The flight model's tests (openspec change `fly-stolen-aircraft`, design section 7), against the shipped data on flat
//! ground: each reads as the behaviour it pins.

use super::*;
use crate::data::load_ron;

const DT: f32 = 1.0 / 60.0;

fn cfg() -> FlightConfig {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/aircraft/flight.ron");
    load_ron(&p).unwrap()
}

fn flat(_: f32, _: f32) -> f32 {
    0.0
}

/// A fixed wing in level flight at its trim speed `alt_m` up, nose at the trim angle, throttle against its drag there.
fn trimmed(spec: &PlaneSpec, alt_m: f32) -> (Body, Controls) {
    let a = spec.trim_alpha(spec.trim_m_s);
    let mut b = Body::resting(Vec3::new(0.0, alt_m, 0.0), 0.0);
    b.on_ground = false;
    b.rot = Quat::from_rotation_x(a);
    b.vel = Vec3::new(0.0, 0.0, -spec.trim_m_s);
    let q = 0.5 * density_kg_m3(alt_m) * spec.trim_m_s * spec.trim_m_s;
    let cl = spec.cl(a, 0.0);
    let drag = q * spec.wing_area_m2 * (spec.cd0 + spec.k_induced * cl * cl);
    let throttle = drag / (spec.thrust_n * density_kg_m3(alt_m) / RHO_SEA_KG_M3);
    (
        b,
        Controls {
            throttle,
            ..Default::default()
        },
    )
}

#[test]
fn every_kind_carries_its_weight_at_its_trim_speed_below_the_stall() {
    for (k, p) in &cfg().planes {
        let a = p.trim_alpha(p.trim_m_s);
        let q = 0.5 * RHO_SEA_KG_M3 * p.trim_m_s * p.trim_m_s;
        let lift = q * p.wing_area_m2 * p.cl(a, 0.0);
        assert!(
            (lift / p.weight_n() - 1.0).abs() < 0.02,
            "{k}: lift {lift} against weight {}",
            p.weight_n()
        );
        assert!(
            a < p.stall_rad(0.0) - 2f32.to_radians(),
            "{k}: trim at {:.1} degrees",
            a.to_degrees()
        );
        assert!(trimmed(p, 0.0).1.throttle < 0.9, "{k} has thrust to spare at its trim speed");
    }
}

#[test]
fn lift_peaks_at_the_critical_angle_and_falls_past_it() {
    for (k, p) in &cfg().planes {
        let s = p.stall_rad(0.0);
        let at = p.cl(s, 0.0);
        assert!(at > p.cl(s - 1f32.to_radians(), 0.0), "{k}");
        assert!(at > p.cl(s + 3f32.to_radians(), 0.0), "{k}");
        assert!(p.cl(s + 8f32.to_radians(), 0.0) < at * 0.9, "{k}: past the stall lift is lost");
        assert!(
            p.cl(p.stall_rad(1.0), 1.0) > at,
            "{k}: flaps raise the peak, though it comes 2 degrees sooner"
        );
        assert!(p.cl(0.05, 1.0) > p.cl(0.05, 0.0), "{k}: flaps add lift");
    }
}

#[test]
fn every_kinds_wheels_are_its_models_own() {
    let c = cfg();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/aircraft");
    let mut checked = 0;
    for (k, rest) in c
        .planes
        .iter()
        .map(|(k, p)| (k, p.rest_m))
        .chain(c.rotors.iter().map(|(k, r)| (k, r.rest_m)))
    {
        let Ok(text) = std::fs::read_to_string(dir.join(format!("{k}.json"))) else {
            continue;
        };
        let m: serde_json::Value = serde_json::from_str(&text).unwrap();
        let want = m["rest_m"].as_f64().unwrap() as f32;
        assert!((rest - want).abs() < 0.01, "{k}: rest_m {rest} against its model's {want}");
        checked += 1;
    }
    assert!(checked >= 8, "every modelled kind: {checked}");
}

#[test]
fn trimmed_and_left_alone_each_plane_holds_level_flight_for_thirty_seconds() {
    for (k, p) in &cfg().planes {
        let (mut b, c) = trimmed(p, 1000.0);
        for _ in 0..(30.0 / DT) as usize {
            step_plane(p, &mut b, c, Assists::default(), Damage::default(), &flat, DT);
        }
        assert!((b.pos.y - 1000.0).abs() < 60.0, "{k} drifted to {:.0} m", b.pos.y);
        let v = b.vel.length();
        assert!((v / p.trim_m_s - 1.0).abs() < 0.2, "{k} at {v:.0} m/s against {}", p.trim_m_s);
    }
}

#[test]
fn the_stall_assist_holds_full_back_stick_under_the_stall_and_without_it_the_wing_stalls() {
    let p = &cfg().planes["light_plane"];
    let mut peak = [0.0f32; 2];
    for (i, stall) in [true, false].into_iter().enumerate() {
        let (mut b, mut c) = trimmed(p, 1500.0);
        c.pitch = 1.0;
        let a = Assists {
            stall,
            ..Default::default()
        };
        for _ in 0..(8.0 / DT) as usize {
            let r = step_plane(p, &mut b, c, a, Damage::default(), &flat, DT);
            peak[i] = peak[i].max(r.alpha_rad);
        }
    }
    assert!(
        peak[0] < p.stall_rad(0.0) - 1f32.to_radians(),
        "assisted: {:.1} degrees",
        peak[0].to_degrees()
    );
    assert!(
        peak[1] > p.stall_rad(0.0),
        "unassisted it stalls: {:.1} degrees",
        peak[1].to_degrees()
    );
}

#[test]
fn a_plane_rolls_down_the_runway_lifts_off_and_climbs() {
    let p = &cfg().planes["propliner"];
    let mut b = Body::resting(Vec3::new(0.0, p.rest_m, 0.0), 0.0);
    let mut c = Controls {
        throttle: 1.0,
        gear_down: true,
        flaps: 0.5,
        ..Default::default()
    };
    let mut lifted = None;
    for i in 0..(90.0 / DT) as usize {
        // rotate at the trim speed's three quarters
        c.pitch = if b.vel.length() > p.trim_m_s * 0.75 { 0.6 } else { 0.0 };
        step_plane(p, &mut b, c, Assists::default(), Damage::default(), &flat, DT);
        if lifted.is_none() && !b.on_ground {
            lifted = Some(i as f32 * DT);
        }
    }
    let t = lifted.expect("it left the ground");
    assert!(t > 5.0 && t < 60.0, "off the ground after {t:.0} s");
    assert!(b.pos.y > 100.0, "climbing: {:.0} m", b.pos.y);
    assert!(b.pos.x.abs() < 50.0, "straight down the runway: {:.0} m off", b.pos.x);
}

#[test]
fn an_engine_lost_on_one_side_yaws_the_plane_toward_it() {
    let p = &cfg().planes["propliner"];
    let (mut b, mut c) = trimmed(p, 1000.0);
    c.throttle = 1.0;
    // the left engine (6 m left of the centre line) is gone
    let d = Damage::default().engine_lost(-6.0, 2);
    assert!((d.thrust - 0.5).abs() < 1e-6);
    assert!(
        (d.thrust_arm_m - 6.0).abs() < 1e-4,
        "what is left pushes from the right: {}",
        d.thrust_arm_m
    );
    for _ in 0..(5.0 / DT) as usize {
        step_plane(p, &mut b, c, Assists::default(), d, &flat, DT);
    }
    let (heading, _, _) = b.attitude();
    assert!(heading > 0.02, "the nose swings left, toward the dead engine: {heading}");
}

#[test]
fn a_helicopter_on_hover_hold_holds_its_place_for_thirty_seconds() {
    for (k, r) in &cfg().rotors {
        let mut b = Body::resting(Vec3::new(0.0, 100.0, 0.0), 0.7);
        b.on_ground = false;
        let c = Controls {
            throttle: 0.5,
            ..Default::default()
        };
        for _ in 0..(30.0 / DT) as usize {
            step_rotor(r, &mut b, c, Assists::default(), Damage::default(), &flat, DT);
        }
        assert!((b.pos.y - 100.0).abs() < 0.5, "{k} at {:.2} m", b.pos.y);
        assert!(Vec2::new(b.pos.x, b.pos.z).length() < 2.0, "{k} drifted {:?}", b.pos);
    }
}

#[test]
fn a_helicopter_lifts_off_with_collective_and_its_cyclic_flies_it_forward() {
    let r = &cfg().rotors["civil_helicopter"];
    let mut b = Body::resting(Vec3::new(0.0, r.rest_m, 0.0), 0.0);
    let off = Assists {
        hover: false,
        ..Default::default()
    };
    let lift = Controls {
        throttle: 0.85,
        ..Default::default()
    };
    for _ in 0..(10.0 / DT) as usize {
        step_rotor(r, &mut b, lift, off, Damage::default(), &flat, DT);
    }
    assert!(b.pos.y > 20.0, "up: {:.1} m", b.pos.y);
    let fwd = Controls { pitch: -0.3, ..lift };
    for _ in 0..(1.0 / DT) as usize {
        step_rotor(r, &mut b, fwd, off, Damage::default(), &flat, DT);
    }
    for _ in 0..(5.0 / DT) as usize {
        step_rotor(r, &mut b, lift, off, Damage::default(), &flat, DT);
    }
    assert!(b.vel.z < -5.0, "nose down, it flies forward (toward -Z): {:?}", b.vel);
}

#[test]
fn a_helicopter_without_its_tail_rotor_spins() {
    let r = &cfg().rotors["military_helicopter"];
    let mut b = Body::resting(Vec3::new(0.0, 200.0, 0.0), 0.0);
    b.on_ground = false;
    let c = Controls {
        throttle: 0.7,
        ..Default::default()
    };
    let d = Damage::from_broken(&["tail_rotor"], 1);
    assert!(!d.tail_rotor);
    for _ in 0..(5.0 / DT) as usize {
        step_rotor(r, &mut b, c, Assists::default(), d, &flat, DT);
    }
    assert!(b.omega.y.abs() > 1.0, "spinning at {:.1} rad/s", b.omega.y);
}

#[test]
fn the_same_controls_fly_the_same_path_and_others_do_not() {
    let p = &cfg().planes["fighter"];
    let fly = |roll: f32| {
        let (mut b, mut c) = trimmed(p, 2000.0);
        c.roll = roll;
        c.pitch = 0.3;
        for _ in 0..(20.0 / DT) as usize {
            step_plane(p, &mut b, c, Assists::default(), Damage::default(), &flat, DT);
        }
        b
    };
    assert_eq!(fly(0.2), fly(0.2));
    assert_ne!(fly(0.2).pos, fly(0.25).pos);
}

#[test]
fn a_broken_zone_takes_its_control_and_its_thrust() {
    let d = Damage::from_broken(&["engine_1", "tailplane_l", "fin"], 4);
    assert!((d.thrust - 0.75).abs() < 1e-6);
    assert!((d.pitch - 0.5).abs() < 1e-6);
    assert_eq!(d.yaw, 0.0);
    assert_eq!(d.roll, 1.0);
    assert!(d.tail_rotor);
}

#[test]
fn the_shipped_flight_data_validates_and_every_kind_flies_in_the_traffic() {
    let c = cfg();
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/air_traffic.ron");
    let t: crate::air_traffic::AirTrafficConfig = load_ron(&p).unwrap();
    for k in c.planes.keys().chain(c.rotors.keys()) {
        let spec = t.kinds.get(k).unwrap_or_else(|| panic!("{k} is no air traffic kind"));
        assert_eq!(
            spec.role.rotorcraft(),
            c.rotors.contains_key(k),
            "{k}: a rotorcraft is flown as one"
        );
    }
    assert_eq!(c.planes.len() + c.rotors.len(), t.kinds.len(), "every kind can be flown");
}
