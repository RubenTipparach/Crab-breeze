//! The audio data checked against the real files (CLAUDE.md 6.5: validate the real artifact).
//!
//! Every cue, bed and music file exists; every placed cue's file is mono, since a stereo file
//! cannot be placed in 3D (owner, 2026-10-25: "verify all sounds are 3d based"), and its muffled
//! and far copies are rendered beside it; every music cue's tempo matches its score's `setcps`.

use cb_core::audio::{AudioData, Variant};
use std::path::Path;

/// The channel count in an Ogg Vorbis file's identification header, if it is one.
pub fn vorbis_channels(bytes: &[u8]) -> Option<u8> {
    let head = &bytes[..bytes.len().min(512)];
    let at = head.windows(7).position(|w| w == b"\x01vorbis")?;
    // packet type and "vorbis" (7 bytes), then a 4-byte version, then the channel count
    head.get(at + 11).copied()
}

/// The tempo a Strudel score sets with `setcps(<bpm>/60/4)`.
pub fn score_bpm(score: &str) -> Option<f32> {
    let at = score.find("setcps(")? + "setcps(".len();
    let rest = &score[at..];
    rest[..rest.find('/')?].trim().parse().ok()
}

/// Every problem with the audio data and its files, one line each.
pub fn check_audio(audio: &AudioData, assets: &Path, music_scores: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (name, cue) in &audio.cues.cues {
        for file in &cue.files {
            let variants: &[Variant] = if cue.space.is_placed() {
                &[Variant::Near, Variant::Muffled, Variant::Far]
            } else {
                &[Variant::Near]
            };
            for v in variants {
                let path = assets.join(v.path(file));
                match std::fs::read(&path) {
                    Err(_) => out.push(format!("cues.{name}: {} is missing", path.display())),
                    Ok(bytes) if cue.space.is_placed() => match vorbis_channels(&bytes) {
                        Some(1) => {}
                        Some(n) => out.push(format!(
                            "cues.{name}: {} has {n} channels; a placed sound must be mono",
                            path.display()
                        )),
                        None => out.push(format!("cues.{name}: {} is not Ogg Vorbis", path.display())),
                    },
                    Ok(_) => {}
                }
            }
        }
    }
    for (name, bed) in &audio.ambience.beds {
        if !assets.join(&bed.file).is_file() {
            out.push(format!("ambience beds.{name}: {} is missing", bed.file));
        }
    }
    for (state, cue) in &audio.music.cues {
        if !assets.join(&cue.file).is_file() {
            out.push(format!("music cues.{state}: {} is missing", cue.file));
        }
        let stem = Path::new(&cue.file).file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        let score = music_scores.join(format!("{stem}.strudel"));
        match std::fs::read_to_string(&score).ok().as_deref().and_then(score_bpm) {
            Some(bpm) if (bpm - cue.bpm).abs() < 0.01 => {}
            Some(bpm) => out.push(format!("music cues.{state}: bpm {} but {} sets {bpm}", cue.bpm, score.display())),
            None => out.push(format!("music cues.{state}: no setcps in {}", score.display())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_channel_count_is_read_from_the_vorbis_header() {
        let mut ogg = b"OggS\0\x02junkjunk".to_vec();
        ogg.extend_from_slice(b"\x01vorbis\0\0\0\0\x01\x44\xac\0\0");
        assert_eq!(vorbis_channels(&ogg), Some(1));
        assert_eq!(vorbis_channels(b"not an ogg"), None);
    }

    #[test]
    fn the_score_tempo_is_read_from_setcps() {
        assert_eq!(score_bpm("// x\nsetcps(150/60/4)\n"), Some(150.0));
        assert_eq!(score_bpm("no tempo"), None);
    }

    #[test]
    fn every_shipped_sound_effect_is_mono_and_every_file_is_there() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = cb_core::GameData::load(&repo.join("data"), "mechs/placeholder.ron").expect("data loads");
        let problems = check_audio(&data.audio, &repo.join("assets"), &repo.join("music"));
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
