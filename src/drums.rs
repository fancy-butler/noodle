// Noodle — drum machine patterns and kits.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Rows: Kick, Snare, Clap, Hat, Open hat, Low tom, Hi tom, Crash

use crate::engine::DRUM_ROWS;

const fn row(s: &str) -> u16 {
    let b = s.as_bytes();
    let mut m = 0u16;
    let mut i = 0;
    while i < b.len() && i < 16 {
        if b[i] == b'x' {
            m |= 1 << i;
        }
        i += 1;
    }
    m
}

const fn pat(r: [&str; DRUM_ROWS]) -> [u16; DRUM_ROWS] {
    [row(r[0]), row(r[1]), row(r[2]), row(r[3]), row(r[4]), row(r[5]), row(r[6]), row(r[7])]
}

/// (name, rows, steps)
pub const PATTERNS: &[(&str, [u16; DRUM_ROWS], u8)] = &[
    ("Empty", [0; DRUM_ROWS], 16),
    (
        "Synthpop",
        pat(["x...x...x...x...", "....x.......x...", "....x.......x...", "x.x.x.x.x.x.x.x.", "..............x.", "", "", "x..............."]),
        16,
    ),
    ("Four on the floor", pat(["x...x...x...x...", "", "....x.......x...", "", "..x...x...x...x.", "", "", ""]), 16),
    ("Rock", pat(["x.......x.x.....", "....x.......x...", "", "x.x.x.x.x.x.x.x.", "", "", "", "x..............."]), 16),
    ("Disco", pat(["x...x...x...x...", "....x.......x...", "", "x.x.x.x.x.x.x.x.", "..x...x...x...x.", "", "", ""]), 16),
    ("Hip-hop", pat(["x......x..x.....", "....x.......x...", "", "x.x.x.x.x.x.x.x.", "", "", "", ""]), 16),
    ("Breakbeat", pat(["x.x.......xx....", "....x..x.x..x..x", "", "x.x.x.x.x.x.x.x.", "", "", "", ""]), 16),
    ("Power ballad", pat(["x.......x.x.....", "....x.......x...", "", "x...x...x...x...", "", "", "", "x..............."]), 16),
    ("Bulgar (3+3+2)", pat(["x.......x.......", "x.....x.....x...", "", "x.x.x.x.x.x.x.x.", "", "", "", ""]), 16),
    ("Hora (3/8 lilt)", pat(["x.......x.......", "......x.......x.", "", "x...x.x.x...x.x.", "", "", "", ""]), 16),
    ("Reel (bodhrán-ish)", pat(["x.......x.......", "", "", "x.x.x.x.x.x.x.x.", "", "....x.......x...", "", ""]), 16),
    ("Jig 6/8", pat(["x.....x.....", "", "", "x.x.x.x.x.x.", "", "......x.....", "", ""]), 12),
    ("Waltz 3/4", pat(["x...........", "....x...x...", "", "x.x.x.x.x.x.", "", "", "", ""]), 12),
];

pub const KITS: &[(&str, u8)] = &[
    ("Standard", 0),
    ("Room", 8),
    ("Power", 16),
    ("Electronic", 24),
    ("TR-808", 25),
    ("Jazz", 32),
    ("Brush", 40),
    ("Orchestra", 48),
];

/// What the 8 pattern slots start as: a groove, its variation, a break, a build, a drop.
pub fn default_slots() -> Vec<(&'static str, [u16; DRUM_ROWS], u8)> {
    let get = |n: &str| PATTERNS.iter().find(|p| p.0 == n).map(|p| (p.1, p.2)).unwrap_or(([0; DRUM_ROWS], 16));
    let (a, al) = get("Synthpop");
    let (b, bl) = get("Four on the floor");
    let (c, cl) = get("Disco");
    let (d, dl) = get("Breakbeat");
    // a breakdown: hats only; a build: snare on every beat then every 8th
    let mut brk = [0u16; DRUM_ROWS];
    brk[3] = a[3];
    brk[7] = 1;
    let mut build = [0u16; DRUM_ROWS];
    build[0] = 0b0001_0001_0001_0001;
    build[1] = 0b0101_0101_0101_0101;
    build[3] = a[3];
    vec![("Synthpop", a, al), ("Four on the floor", b, bl), ("Disco", c, cl), ("Breakbeat", d, dl), ("Breakdown", brk, 16), ("Build", build, 16), ("Empty", [0; DRUM_ROWS], 16), ("Empty", [0; DRUM_ROWS], 16)]
}
