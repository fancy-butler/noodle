// Noodle — Ocarina mode: the window becomes one big ocarina that shows which holes
// to cover for every note you play (or the song plays), plus the next one.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// A child module of `ui`, so it can use App's fields and the ui helpers.

use super::*;
use crate::fingering::{lookup, parse, CHARTS};

/// Glazes for the ocarina picture: (name, colour).
pub const GLAZES: [(&str, [u8; 3]); 5] = [("Blue", [0x2f, 0x5f, 0xb0]), ("Terracotta", [0xb8, 0x5c, 0x38]), ("Celadon", [0x6f, 0xa8, 0x8c]), ("Black", [0x2a, 0x2a, 0x30]), ("Purple", [0x6a, 0x48, 0xa8])];

impl App {
    /// Lowest and highest sounding note of the chosen ocarina (alto C: A4–F6), in Ocarina mode.
    pub(super) fn oc_range(&self) -> Option<(u8, u8)> {
        if !self.oc_mode {
            return None;
        }
        let c = &CHARTS[self.oc_chart()];
        let tr = c.trans.get(self.s.fing_tr.min(c.trans.len().saturating_sub(1))).map(|t| t.1).unwrap_or(0) as i32;
        let lo = c.fings.iter().map(|f| f.midi).min().unwrap_or(69) as i32 + tr;
        let hi = c.fings.iter().map(|f| f.midi).max().unwrap_or(89) as i32 + tr;
        Some((lo.clamp(0, 127) as u8, hi.clamp(0, 127) as u8))
    }

    /// Computer keys: put A (the first key) on the ocarina's C, the first C at or above its lowest note.
    fn oc_place_keys(&mut self) {
        if let Some((lo, _)) = self.oc_range() {
            let c = (lo as i32 + 11) / 12 * 12;
            self.s.kb_octave = (c / 12 - 1).clamp(0, 8);
        }
    }

    fn oc_chart(&self) -> usize {
        CHARTS.iter().position(|c| c.name.starts_with("Ocarina")).unwrap_or(0)
    }

    pub(super) fn enter_ocarina(&mut self) {
        if self.oc_mode {
            return;
        }
        self.oc_mode = true;
        self.oc_prev = Some(self.s.main);
        self.oc_prev_oct = self.s.kb_octave;
        self.s.fing_any = !self.s.real_range;
        // the recorded ocarina if it's there, otherwise the General MIDI one (the header offers the download)
        let bp = if self.s.oc_bank { (0, 79) } else { self.best_sound(79) };
        self.set_preset(false, bp);
        let c = self.oc_chart();
        if self.s.fing_idx != c {
            self.s.fing_idx = c;
            self.s.fing_tr = 0;
        }
        self.oc_place_keys();
        self.oc_tr_seen = self.s.fing_tr;
    }

    pub(super) fn exit_ocarina(&mut self) {
        if !self.oc_mode {
            return;
        }
        self.oc_mode = false;
        self.s.kb_octave = self.oc_prev_oct;
        if let Some(bp) = self.oc_prev.take() {
            self.set_preset(false, bp);
        }
        // back to the keyboard's own Play as choice
        if let Some(i) = self.s.kb_inst.as_deref().and_then(|n| CHARTS.iter().position(|c| c.name == n)) {
            self.s.fing_idx = i;
            self.s.fing_tr = self.s.kb_inst_tr;
            self.play_seen = (i, self.s.kb_inst_tr);
        }
        self.s.fing_any = self.s.kb_inst.is_none();
    }

    pub(super) fn oc_screen(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let ci = self.oc_chart();
        let chart = &CHARTS[ci];
        // ---- one header line: ocarina key, glaze, any-octave ----
        let top = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 32.0));
        let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::left_to_right(egui::Align::Center)));
        {
            let ui = &mut bar;
            ui.label(RichText::new("12-hole ocarina").size(17.0).strong());
            let tr = chart.trans;
            let ti = self.s.fing_tr.min(tr.len() - 1);
            egui::ComboBox::from_id_salt("oc_key").selected_text(tr[ti].0).width(110.0).show_ui(ui, |ui| {
                for (i, t) in tr.iter().enumerate() {
                    ui.selectable_value(&mut self.s.fing_tr, i, t.0);
                }
            });
            if let Some((lo, hi)) = self.oc_range() {
                let base = ((self.s.kb_octave + 1) * 12) as u8;
                ui.label(RichText::new(format!("Plays {}–{}  ·  lowest = every hole covered  ·  computer key A = {}", theory::note_name_oct(lo), theory::note_name_oct(hi), theory::note_name_oct(base))).size(12.5).color(fg()));
            }
            ui.separator();
            self.range_switch(ui);
            ui.separator();
            // which ocarina you hear: the recording (VCSL) or the sound bank's
            ui.label(RichText::new("Sound").size(12.0).color(text_dim()));
            let rec = !self.s.oc_bank;
            let have = crate::packs::better_for_gm(79).map(crate::packs::installed).unwrap_or(false);
            if have && ui.selectable_label(rec, "Recorded").on_hover_text("A real ocarina recorded with vibrato (VCSL, CC0)").clicked() {
                self.s.oc_bank = false;
                let bp = self.best_sound(79);
                self.set_preset(false, bp);
            }
            if ui.selectable_label(!rec || !have, format!("{}", self.bank_label())).on_hover_text("The sound bank's ocarina: plainer, no vibrato").clicked() {
                self.s.oc_bank = true;
                self.set_preset(false, (0, 79));
            }
            ui.separator();
            ui.label(RichText::new("Glaze").size(12.0).color(text_dim()));
            for (i, (n, c)) in GLAZES.iter().enumerate() {
                let on = self.s.oc_glaze as usize == i;
                let b = egui::Button::new(RichText::new(*n).color(Color32::WHITE)).fill(Color32::from_rgb(c[0], c[1], c[2])).stroke(Stroke::new(if on { 2.5 } else { 0.0 }, YOURS));
                if ui.add(b).clicked() {
                    self.s.oc_glaze = i as u8;
                }
            }

        }
        if self.oc_tr_seen != self.s.fing_tr {
            self.oc_tr_seen = self.s.fing_tr;
            self.oc_place_keys();
        }
        let tr: i8 = chart.trans.get(self.s.fing_tr.min(chart.trans.len().saturating_sub(1))).map(|t| t.1).unwrap_or(0);
        let held = (0..128u8).rev().find(|k| self.held[0][*k as usize] > 0);
        let now_key = held.or(self.last_live).or_else(|| std::env::var("NOODLE_OC_NOTE").ok().and_then(|v| v.parse().ok()));
        let next_key = self.next_target();
        let gl = GLAZES[(self.s.oc_glaze as usize).min(GLAZES.len() - 1)].1;
        let glaze = Color32::from_rgb(gl[0], gl[1], gl[2]);
        let area = Rect::from_min_max(Pos2::new(rect.left(), top.bottom() + 6.0), rect.max);
        // big ocarina on the left, the next note smaller on the right
        let split = area.left() + area.width() * 0.72;
        let big = Rect::from_min_max(area.min, Pos2::new(split - 8.0, area.bottom()));
        let small = Rect::from_min_max(Pos2::new(split + 8.0, area.top() + area.height() * 0.25), Pos2::new(area.right(), area.bottom() - area.height() * 0.15));
        let p = ui.painter_at(rect);
        p.rect_filled(big, 16.0, card_bg());
        p.rect_filled(small, 14.0, card_bg().gamma_multiply(0.7));
        let speller = self.speller();
        let mut note_for = |key: Option<u8>| -> (String, Option<Vec<u8>>, bool) {
            match key {
                None => (String::new(), None, false),
                Some(k) => {
                    let name = if speller.name(k % 12).is_empty() { theory::note_name_oct(k) } else { theory::note_name_oct(k) };
                    match lookup(chart, k, self.s.fing_any, tr) {
                        Some((e, exact)) => (format!("{name}{}", if exact { "" } else { "  (8va)" }), Some(parse(e.states)), e.check),
                        None => (format!("{name} — not on this ocarina"), None, false),
                    }
                }
            }
        };
        let (now_name, now_st, now_q) = note_for(now_key);
        let (next_name, next_st, _) = note_for(next_key);
        // big picture
        let name_h = (big.height() * 0.16).clamp(28.0, 90.0);
        let title = if now_key.is_none() { "play a note".to_string() } else { format!("{now_name}{}", if now_q { "  ?" } else { "" }) };
        p.text(Pos2::new(big.center().x, big.top() + 10.0), Align2::CENTER_TOP, title, FontId::proportional(name_h * 0.8), if now_q { YOURS } else { fg() });
        let pic = Rect::from_min_max(Pos2::new(big.left() + 30.0, big.top() + name_h + 20.0), Pos2::new(big.right() - 30.0, big.bottom() - 40.0));
        crate::pipe_art::draw_ocarina(&p, pic, now_st.as_deref(), 1.0, glaze);
        let foot = if now_q { "? = Pure Ocarinas' two charts disagree on this one — check it on yours" } else { "Subholes: slide the middle finger forward over both holes  ·  half-filled = half-cover  ·  small view = the underside, where your thumbs go" };
        p.text(Pos2::new(big.center().x, big.bottom() - 26.0), Align2::CENTER_CENTER, foot, FontId::proportional(12.0), text_dim());
        p.text(Pos2::new(big.center().x, big.bottom() - 10.0), Align2::CENTER_CENTER, format!("Fingerings: {}", chart.source), FontId::proportional(10.5), text_dim().gamma_multiply(0.7));
        // next
        if next_key.is_some() {
            p.text(Pos2::new(small.center().x, small.top() + 10.0), Align2::CENTER_TOP, format!("next  {next_name}"), FontId::proportional(20.0), text_dim());
            let pr = Rect::from_min_max(Pos2::new(small.left() + 14.0, small.top() + 44.0), Pos2::new(small.right() - 14.0, small.bottom() - 14.0));
            crate::pipe_art::draw_ocarina(&p, pr, next_st.as_deref(), 0.55, glaze);
        } else {
            p.text(small.center(), Align2::CENTER_CENTER, "Load a tune (Walk it, Game\nor a song) and the next\nnote shows up here", FontId::proportional(13.0), text_dim());
        }
    }
}
