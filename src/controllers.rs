// Noodle — ready-made mappings for common MIDI controllers' knobs and faders.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// The CC numbers are each controller's factory defaults as published in Spectrasonics'
// Omnisphere 3 Hardware Guide (support.spectrasonics.net), which documents them per model.
// Keys, velocity, pitch bend, mod wheel and sustain work on every controller without this;
// these add the knobs. Anything else: right-click a knob in Noodle → Learn.

use crate::engine::target::*;

pub struct Preset {
    pub name: &'static str,
    /// lowercase, spaces removed, matched against the MIDI port name
    pub ids: &'static [&'static str],
    /// knob CCs, in order → Filter, Reso, Drive, Delay, Reverb, Repeats, Chorus, Arp gate
    pub knobs: &'static [u8],
    /// fader CCs, in order → Volume, Layer, Drums, Song
    pub faders: &'static [u8],
    /// what to set on the controller, if anything
    pub note: &'static str,
}

pub const KNOB_TARGETS: [u8; 8] = [FILTER, RESO, DRIVE, DELAY, REVERB, REPEATS, CHORUS, ARP_GATE];
pub const FADER_TARGETS: [u8; 4] = [MASTER, LAYER_VOL, DRUM_VOL, SONG_VOL];

/// More specific names first (Oxygen Pro before Oxygen, Axiom Pro before Axiom).
pub const PRESETS: &[Preset] = &[
    Preset { name: "Akai MPK Mini (mk3, Play, Plus)", ids: &["mpkmini", "mpk mini"], knobs: &[70, 71, 72, 73, 74, 75, 76, 77], faders: &[], note: "Program 1 (MPC), as it powers up" },
    Preset {
        name: "Novation Launchkey (Mini / 25–88, mk1–mk4)",
        ids: &["launchkey"],
        knobs: &[21, 22, 23, 24, 25, 26, 27, 28],
        faders: &[79, 71, 72, 73],
        note: "Knobs: Custom pot mode (Shift + the pad marked Custom 1). Faders on the 49/61/88.",
    },
    Preset {
        name: "Arturia KeyLab / KeyLab Essential",
        ids: &["keylab"],
        knobs: &[74, 71, 76, 77, 93, 18, 19, 16],
        faders: &[85, 73, 75, 79],
        note: "A User template with factory settings (not DAW mode)",
    },
    Preset { name: "M-Audio Oxygen Pro 25/49/61", ids: &["oxygenpro"], knobs: &[44, 45, 46, 47, 62, 63, 75, 76], faders: &[], note: "" },
    Preset { name: "M-Audio Axiom Pro 25/49/61", ids: &["axiompro"], knobs: &[10, 2, 12, 13, 75, 76, 92, 95], faders: &[7, 74, 71, 91], note: "" },
    Preset {
        name: "M-Audio Axiom / Oxygen 25–61 (older)",
        ids: &["axiom", "oxygen"],
        knobs: &[75, 76, 92, 95, 10, 2, 12, 13],
        faders: &[74, 71, 91, 93],
        note: "Factory preset 1",
    },
    Preset { name: "M-Audio Keystation (any)", ids: &["keystation"], knobs: &[], faders: &[7], note: "Volume slider/knob → Noodle's volume" },
    Preset { name: "Korg nanoKONTROL2", ids: &["nanokontrol2"], knobs: &[16, 17, 18, 19, 20, 21, 22, 23], faders: &[0, 1, 2, 3], note: "CC mode, factory scene" },
    Preset { name: "Native Instruments Komplete Kontrol A25/A49/A61", ids: &["kompletekontrola", "kontrola25", "kontrola49", "kontrola61"], knobs: &[14, 15, 16, 17, 18, 19, 20, 21], faders: &[], note: "MIDI mode (Shift + Plug-in)" },
    Preset { name: "IK Multimedia iRig Keys IO", ids: &["irigkeysio"], knobs: &[12, 13, 14, 15, 16, 17, 18, 19], faders: &[], note: "" },
];

/// The preset for a connected MIDI port, by name.
pub fn detect(port: &str) -> Option<usize> {
    let l = port.to_lowercase().replace([' ', '-', '_'], "");
    PRESETS.iter().position(|p| p.ids.iter().any(|id| l.contains(&id.replace(' ', ""))))
}

/// (cc, target) pairs for a preset.
pub fn map(p: &Preset) -> Vec<(u8, u8)> {
    p.knobs.iter().zip(KNOB_TARGETS).chain(p.faders.iter().zip(FADER_TARGETS)).map(|(c, t)| (*c, t)).collect()
}

/// Which target a knob in Noodle's window stands for (for Learn).
pub fn target_for_label(label: &str) -> Option<u8> {
    Some(match label {
        "Filter" => FILTER,
        "Reso" => RESO,
        "Drive" => DRIVE,
        "Delay" => DELAY,
        "Reverb" => REVERB,
        "Repeats" => REPEATS,
        "Chorus" => CHORUS,
        "Gate" => ARP_GATE,
        "Volume" => MASTER,
        "Swing" => SWING,
        "Level" => DRUM_VOL,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ports_are_recognised() {
        assert_eq!(detect("MPK mini 3 MIDI 1").map(|i| PRESETS[i].name), Some(PRESETS[0].name));
        assert_eq!(detect("Launchkey Mini MK3 MIDI 1").map(|i| PRESETS[i].knobs[0]), Some(21));
        assert_eq!(detect("Oxygen Pro 49 USB MIDI").map(|i| PRESETS[i].knobs[0]), Some(44), "Oxygen Pro before Oxygen");
        assert_eq!(detect("USB Axiom 49 Port 1").map(|i| PRESETS[i].knobs[0]), Some(75));
        assert_eq!(detect("Axiom Pro 49 MIDI").map(|i| PRESETS[i].knobs[0]), Some(10));
        assert_eq!(detect("nanoKONTROL2 nanoKONTROL2 _ CTRL").map(|i| PRESETS[i].faders[0]), Some(0));
        assert_eq!(detect("Keystation 49es"), detect("Keystation 61 MK3"));
        assert_eq!(detect("Minilab3 MIDI"), None, "MiniLab 3 has its own mapping");
        for p in PRESETS {
            let m = map(p);
            let mut ccs: Vec<u8> = m.iter().map(|x| x.0).collect();
            ccs.sort();
            ccs.dedup();
            assert_eq!(ccs.len(), m.len(), "{}: a CC used twice", p.name);
        }
    }
}
