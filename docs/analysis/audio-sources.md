# Audio sources: soundfonts, sound effects, tools and temp music

What a search found on 2026-10-25 for Crab Breeze's music and sound, starting from the lists the
owner gave: [FronkonGames/Awesome-Gamedev](https://github.com/FronkonGames/Awesome-Gamedev)
**[FG]**, [Calinou/awesome-gamedev](https://github.com/Calinou/awesome-gamedev) **[CA]** and
[OpenGameArt](https://opengameart.org/) **[OGA]**, plus a web search **[web]**. Nothing here is in
the repository except GeneralUser GS, which `tools/music/fetch_soundfonts.sh` downloads.

**How far the licences are checked.** The cloud session's network refuses opengameart.org,
freesound.org, kenney.nl, musescore.org, schristiancollins.com and musical-artifacts.com. Licences
marked **(page)** were read on the project's own page or repository; **(snippet)** came from a
search engine's extract of the page and must be opened on the page before a file ships. Survey P4
decides which licences we use (recommended: CC0, CC-BY with a credits screen where nothing CC0
fits). `openspec/changes/audio` holds the sound effect plan and its first OpenGameArt finds.

What [FG] gave: mostly talks and tutorials; its resource links are Freesound, the Sonniss GDC
bundles, as3sfxr, signaturesounds.org and conservethesound.de. [CA] gave licensed tools, sound
sources and music archives.

## Soundfonts (for LMMS's Sf2Player and fluidsynth)

LMMS 1.2 reads sf2 (sf3 only on some builds) and cannot load SFZ.

| Name | Format | Licence | Where | Fit |
| --- | --- | --- | --- | --- |
| **GeneralUser GS 2.0.3** (S. Christian Collins) | sf2, 32 MB | GeneralUser GS License v2.0 (page): "use without restriction for your own music creation, private or commercial" | https://github.com/mrbumpy409/GeneralUser-GS | **In use** (`music/instruments.json`). Asks not to hotlink its files: we fetch from GitHub to `/opt/soundfonts` by pinned commit. |
| FluidR3_GM | sf2, 141 MB | MIT (snippet) | Debian `fluid-soundfont-gm` | The old default; still installed for `--preview` comparisons. |
| MuseScore_General | sf3 36 MB, sf2 208 MB | MIT (snippet) | ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/ (path unverified) | A good second GM set to compare orchestral parts. |
| VSCO 2 Community Edition | SFZ + WAV | CC0 (page) | https://github.com/sgossner/VSCO-2-CE/releases/tag/1.1.0 | The best free orchestra with clean rights; needs Polyphone (to sf2) or sfizz. |
| Sonatina Symphonic Orchestra | SFZ, community sf2 | CC Sampling Plus 1.0 (snippet) | widely mirrored | Retired licence, conversions unconfirmed: avoid. |
| Virtual Playing Orchestra 3 | SFZ, 842 MB | mixed, including CC BY-SA (snippet) | http://virtualplaying.com | Needs a licence decision. |
| Soundfonts by rabid47 | sf2 collection | CC0 (snippet) | https://archive.org/details/Soundfonts | Unknown quality. |
| 80s drums v2, DX Sound | sf2 | give credit / CC BY 3.0 (snippet) | polyphone.io, musical-artifacts.com | No CC0 80s synth bank found; LMMS's own synths are ours outright. |

## Sound effects

| Name | Covers | Where | Licence |
| --- | --- | --- | --- |
| Mech Stomp / Step Sound [OGA] | mech footstep | https://opengameart.org/content/mech-stomp-step-sound | CC0 (snippet) |
| Metal footsteps on concrete [OGA] | 25 metal steps | https://opengameart.org/content/metal-footsteps-on-concrete | CC0 (snippet) |
| Steam release sounds (bart) [OGA] | steam hiss for the boarding | https://opengameart.org/content/steam-release-sounds | CC0 (snippet) |
| **Audioaugust (Inexor)** [CA] | hydraulic compressor, robotics, turbines, gates, levers, doors, steps, impacts, ventilation | https://github.com/inexor-game/audioaugust | CC-BY 4.0 (page): credit "Inexor". The best servo and hydraulic set found. |
| 100 CC0 SFX, 100 CC0 SFX #2 [OGA] | metal, slams, springs, machine loops, doors | https://opengameart.org/content/100-cc0-sfx, https://opengameart.org/content/100-cc0-sfx-2 | CC0 (snippet) |
| 30 CC0 SFX loops [OGA] | machine loops (cockpit hum, skate motors) | https://opengameart.org/content/30-cc0-sfx-loops | CC0 (snippet) |
| Kenney Sci-Fi Sounds [OGA] | engines, explosions | https://opengameart.org/content/sci-fi-sounds | CC0 (snippet) |
| Kenney Interface Sounds [OGA] | UI clicks for the MFD buttons | https://github.com/Calinou/kenney-interface-sounds | CC0 (snippet) |
| Missile sound [OGA] | missile launch | https://opengameart.org/content/missile-sound | CC0 (snippet) |
| The Free Firearm Sound Library [OGA] | gunfire (pitch down for the autocannon) | https://opengameart.org/content/the-free-firearm-sound-library | CC0 (snippet) |
| 25 CC0 bang / firework SFX [OGA] | explosions, cannon | https://opengameart.org/content/25-cc0-bang-firework-sfx | CC0 (snippet) |
| Interface beeps; Atmospheric Interaction Sound Pack [OGA] | beeps, warnings (raw material for lock and alarm tones) | https://opengameart.org/content/interface-beeps, https://opengameart.org/content/atmospheric-interaction-sound-pack | CC0 (snippet) |
| Frequency Static Sound Effects [OGA] | radio static and crackle | https://opengameart.org/content/frequency-static-sound-effects | CC0 (snippet) |
| Sonniss GDC Game Audio Bundle [FG] | tens of GB of professional jets, explosions, machinery | https://sonniss.com/gameaudiogdc | Custom royalty-free (snippet): commercial use, no credit, no resale; recent years forbid AI training. Read that year's licence. |
| Freesound [FG][CA] | everything | https://freesound.org | per sound: filter to CC0, avoid NC |

Gaps: no confirmed CC0 lock-on or radar ping, jet loop or wheel-skate loop. Synthesise the tones
(jsfxr, below; the lock tone can take the Call Sign rhythm, `docs/design/music.md`) and take the
engines from a Sonniss bundle.

## Recorded CC0 replacements (owner, 2026-10-25: "can you not pull from cc0 sound fx?")

The synthesised cues stay in place until each one has a recording that sounds better. A recorded
cue is cut from CC0 originals by a committed spec (layers with start, length, pitch, gain, lowpass
and fades, plus synthesised layers such as the sub rumble where a recording lacks low end), and
gets a row in `assets/audio/sfx/SOURCES.md`.

**Blocked on the network.** The cloud environment refuses opengameart.org, kenney.nl, archive.org,
commons.wikimedia.org, upload.wikimedia.org,
freesound.org, godotengine.org and mediafire.com, where every original lives. The GitHub mirrors
found on 2026-10-25: [hipdiscovery/Free-Sound-Effects](https://github.com/hipdiscovery/Free-Sound-Effects)
holds only Kenney's 100 Interface Sounds (CC0, licence file checked); the Godot Asset Library's
Kenney ports ([Kenney's Sci-Fi Sounds](https://godotengine.org/asset-library/asset/1834), Impact
Sounds, RPG Audio, UI Audio, from `github.com/Loppansson/*-for-godot`) are gone from GitHub;
`PanderMusubi/sound-effects-library-weapons` is a packaging script whose files sit on mediafire.
[bonesvt/GodotFall](https://github.com/bonesvt/GodotFall) (revision `ad59aac`) ships 226 cuts from
CC0 packs with a source table and its cut specs; the repository has no licence, so its cuts are not
used, but its table and specs say which original file makes a good mech step, explosion or rocket.

| Cue | CC0 originals (OpenGameArt unless marked) | Notes |
| --- | --- | --- |
| foot_stomp | Mech Stomp / Step Sound (hc), `stomp.flac`; 100 CC0 metal and wood SFX (dull metal collisions) | pitched 0.8-0.9 for a 24 m mech, our sub thump under it |
| jet_ignite | Flare ignition (qubodup); Rocket launch (qubodup) | our sub rumble under it |
| rocket_loop | rocket engine (loopable); 30 CC0 SFX loops; NASA launch recordings (below) | steep lowpass as now |
| rocket_rumble | Low rumbling (Musheran) | |
| rocket_launch | Rocket launch (qubodup), `launch.wav` | |
| jet_ignite, rocket_loop, rocket_rumble (real launches) | [NASA Audio Highlight Reels](https://archive.org/details/NasaAudioHighlightReels) on the Internet Archive: `Launch-Sound_Saturn-V.ogg`, `Launch-Sound_Shuttle-Launch-Mid.ogg` and more, in ogg and flac | NASA works are public domain (the item carries the Public Domain Mark); check each file's page. Real ignition and roar, the owner's "fuel igniting". Wikimedia Commons (owner's suggestion) had no launch audio by search, only launch videos whose soundtracks could be cut; both hosts are refused here too |
| missile_launch | Missile sound (mikeask), `missile.wav` | |
| autocannon, cannon, shotgun | The Free Firearm Sound Library (Still North Media): Carl Gustav M45, AK-47, shotgun takes | autocannon pitched about 0.7 |
| impact_mech | 75 CC0 breaking / falling / hit SFX; 100 CC0 metal and wood SFX (`metal_slam`); a Free Firearm Sound Library crack | the owner asked for "a harder sound like gunshot" |
| impact_building | 75 CC0 breaking / falling / hit SFX (rock hits) | |
| explosion_small, explosion_large | Explosions (EZduzziteh); Chunky Explosion (Joth); Mechanical Explosion (Spring Spring) | |
| collapse | 75 CC0 breaking / falling / hit SFX; Low rumbling | |
| hatch_steam | Steam release sounds (bart); Compressed Gas Leak SFX (0new4y) | |
| hatch_seal, hatch_motor, servo_torso, hydraulic | Mechanical Sounds (BMacZero); Robotic Transformations (Mekaal) | |
| laser_loop, laser_stop, rail_charge, rail_fire, plasma_fire, plasma_burst | Sci-Fi Sounds (Kenney); Doomsday Laser Cannon (TAD); Electricity Game Sound Pack (faxcorp) | |
| reactor_hum | Generator (loop) (YCbCr) | |
| radio_squelch | Misc/menu sci-fi sounds (HaelDB); Radio Noise 1 | |
| mfd_click | Interface Sounds (Kenney), reachable now through the GitHub mirror above | |
| lock_tone, lock_on, heat_lock, alarm_warning, radar_ping, systems_boot | none found | stay synthesised (the owner liked the lock sound once softened) |

## Tools

| Name | What | Licence | Fit |
| --- | --- | --- | --- |
| LMMS 1.2 (with ZynAddSubFX, TripleOscillator, Sf2Player) | DAW, renders headless | GPL | **In use** for the final comp. |
| fluidsynth | soundfont player | LGPL | **In use** for `--preview`. |
| jsfxr [CA] | sfxr in JavaScript, runs in Node | Unlicense | Beeps, pings, lock tones, headless. |
| rFXGen [CA], Glasan-FX [CA], sfxr [CA] | sound effect generators | zlib, MIT, MIT | Alternatives to jsfxr. |
| Surge XT | hybrid synth with Python bindings | GPL-3.0 (page) | Rich 80s pads rendered headless as stems. |
| Polyphone | sf2/sf3/SFZ editor and converter | GPL | The bridge from VSCO 2 (SFZ) to sf2. |
| sfizz | SFZ player with a render CLI | BSD-2 | Render SFZ orchestra stems from our MIDI. |
| bevy_kira_audio 0.27 | Bevy plugin on kira; supports Bevy 0.20 (page) | MIT / Apache-2.0 | Candidate for in-game music (crossfades, clocks): `openspec/changes/music`. |
| kira | Rust game audio: tweens, mixer, clocks | MIT / Apache-2.0 | Underneath bevy_kira_audio. |
| oddio | Rust spatial audio with propagation delay | MIT / Apache-2.0 | Fits the 343 m/s delay in `openspec/changes/audio`. |
| Steam Audio | HRTF, occlusion, reverb (C API) | Apache-2.0 | Later: the sealed cockpit, city reflections. |
| FMOD, Wwise | middleware | proprietary, free under budget caps | Heavy for Rust; not proposed. |

## Temp and reference music (CC0)

| Track | Style | Where |
| --- | --- | --- |
| Boss Battle Music (SubspaceAudio) [OGA] | orchestral, epic | https://opengameart.org/content/boss-battle-music |
| Battle Theme A (cynicmusic) [OGA] | strings and horns | https://opengameart.org/content/battle-theme-a |
| Free Music Pack (Alexander Ehlers) [OGA] | orchestral, rock | https://opengameart.org/content/free-music-pack |
| Free Music [OGA] | rock and metal | https://opengameart.org/content/free-music-0 |
| Factory ambiance [OGA] | industrial ambience | https://opengameart.org/content/factory-ambiance |
| Eyeless (Retrowave) [OGA] | synthwave (pick its CC0 option) | https://opengameart.org/content/eyeless-retrowave |
| CC0 Music archive (wowa) [CA] | assorted | http://www.wowa.me/archive |

All CC0 by search snippet; open each page before using one. Avoid "Orchestral Epic Fantasy Music"
(CC-BY-SA 3.0 and GPL) and "Battle March" (CC-BY 3.0) unless P4 allows them.
