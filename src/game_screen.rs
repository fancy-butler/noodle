// Noodle — Game mode as its own screen: setup menu → play → results.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// A child module of `ui`, so it can use App's fields and the ui helpers.

use super::*;

const CARD_W: f32 = 430.0;

fn chip(ui: &mut egui::Ui, on: bool, label: &str) -> bool {
    let txt = RichText::new(label).size(14.0).color(if on { Color32::BLACK } else { fg() });
    ui.add(egui::Button::new(txt).fill(if on { YOURS } else { card_hi() }).corner_radius(12.0).min_size(Vec2::new(0.0, 28.0))).clicked()
}

fn panel_card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(card_bg()).corner_radius(14.0).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.set_width(CARD_W);
            ui.label(RichText::new(title).size(13.0).strong().color(text_dim()));
            ui.add_space(6.0);
            add(ui);
        });
    });
}

fn grade_col(g: &str) -> Color32 {
    match g {
        "S+" | "S" => YOURS,
        "A" => GOOD,
        "B" => ARP,
        _ => BAD,
    }
}

impl App {
    /// Whole central area while Game mode is on.
    pub(super) fn game_screen(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let finished = self.game.as_ref().map(|g| g.finished).unwrap_or(false);
        if self.game.is_some() && !finished {
            self.game_play(ui, rect);
        } else if finished {
            self.game_results(ui, rect);
        } else {
            self.game_menu(ui, rect);
        }
    }

    pub(super) fn exit_game(&mut self) {
        let _ = self.tx.send(Cmd::SongPlay(false));
        self.game = None;
        self.game_mode = false;
        self.show_results = false;
    }

    fn game_menu(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect));
        egui::ScrollArea::vertical().id_salt("game_menu").show(&mut ui, |ui| {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("🎮  GAME").size(30.0).strong().color(YOURS));
                ui.add_space(12.0);
                ui.label(RichText::new("Notes fall onto your keys. Hit them on time.  Perfect ±45 ms · Great ±90 ms · Good ±140 ms").size(13.0).color(text_dim()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(RichText::new("✕  Exit game").size(14.0)).corner_radius(10.0)).on_hover_text("Esc").clicked() {
                        self.exit_game();
                    }
                });
            });
            ui.add_space(14.0);
            ui.horizontal_top(|ui| {
                let total = 2.0 * (CARD_W + 36.0) + 16.0;
                ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                self.menu_drill(ui);
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    self.menu_tune(ui);
                    ui.add_space(16.0);
                    self.menu_bests(ui);
                });
            });
        });
    }

    fn menu_drill(&mut self, ui: &mut egui::Ui) {
        panel_card(ui, "SCALE DRILL", |ui| {
            if self.s.g_pattern != 6 {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("gm_root").selected_text(RichText::new(ROOTS[self.s.scale_root].0).size(16.0)).width(60.0).show_ui(ui, |ui| {
                        for (i, r) in ROOTS.iter().enumerate() {
                            if ui.selectable_value(&mut self.s.scale_root, i, r.0).clicked() {
                                self.rebuild_walk();
                            }
                        }
                    });
                    let sidx = self.s.scale_idx.min(SCALES.len() - 1);
                    egui::ComboBox::from_id_salt("gm_scale").selected_text(RichText::new(SCALES[sidx].name).size(16.0)).width(300.0).height(420.0).show_ui(ui, |ui| {
                        let mut group = "";
                        for (i, sc) in SCALES.iter().enumerate() {
                            if sc.group != group {
                                group = sc.group;
                                ui.label(RichText::new(group).size(11.0).color(text_dim()));
                            }
                            if ui.selectable_value(&mut self.s.scale_idx, i, sc.name).clicked() {
                                self.rebuild_walk();
                            }
                        }
                    });
                });
                ui.add_space(8.0);
            }
            ui.label(RichText::new("Pattern").color(text_dim()));
            ui.horizontal_wrapped(|ui| {
                for (i, n) in game::PATTERNS.iter().enumerate() {
                    if chip(ui, self.s.g_pattern == i, n) {
                        self.s.g_pattern = i;
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Rhythm").color(text_dim()));
            ui.horizontal_wrapped(|ui| {
                for (i, n) in game::RHYTHMS.iter().enumerate() {
                    if chip(ui, self.s.g_rhythm == i, n) {
                        self.s.g_rhythm = i;
                    }
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Tempo").color(text_dim()));
                ui.spacing_mut().slider_width = 260.0;
                ui.add(egui::Slider::new(&mut self.s.g_bpm, 40.0..=220.0).step_by(1.0).fixed_decimals(0).suffix(" bpm"));
            });
            ui.horizontal(|ui| {
                if self.s.g_pattern < 6 {
                    ui.label(RichText::new("Range").color(text_dim()));
                    for o in 1..=2u8 {
                        if chip(ui, self.s.g_octaves == o, &format!("{o} octave{}", if o > 1 { "s" } else { "" })) {
                            self.s.g_octaves = o;
                        }
                    }
                }
                if self.s.g_pattern >= 4 {
                    ui.add_space(8.0);
                    ui.add(egui::DragValue::new(&mut self.s.g_bars).range(1..=32).suffix(" bars"));
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.s.g_any_octave, "Any octave counts").on_hover_text("Right note in any octave scores — handy on a 25-key keyboard");
                ui.toggle_value(&mut self.s.g_hear, "Hear the targets").on_hover_text("Play the target notes too. Off = only you.");
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if big_toggle(ui, true, "▶  START", GOOD, Vec2::new(150.0, 40.0)).on_hover_text("One bar of count-in clicks, then go (Enter)").clicked() {
                    self.start_game(false);
                }
                if let Some(b) = self.s.g_best.get(&self.game_key()) {
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("best  {}  ·  {:.1}%", b.1, b.0 * 100.0)).size(15.0).color(grade_col(&b.1)));
                }
            });
        });
    }

    fn menu_tune(&mut self, ui: &mut egui::Ui) {
        panel_card(ui, "PLAY A TUNE OR SONG", |ui| {
            let playable = self.song.clone().filter(|s| s.name != "Scale" && !s.name.contains(" · "));
            match playable {
                None => {
                    ui.label("Load a tune from The Session, an ABC tune or a MIDI song, then play it here as a game.");
                }
                Some(song) => {
                    ui.label(RichText::new(&song.name).size(17.0).strong());
                    let parts: Vec<u16> = (0..16u16).filter(|ch| song.channels & (1 << ch) != 0 && *ch != 9).collect();
                    if parts.len() == 1 {
                        // single-line tunes (ABC): that's your part
                        if self.song_user != 1 << parts[0] {
                            self.song_user = 1 << parts[0];
                            self.p.song_user.store(self.song_user, Relaxed);
                        }
                        ui.label(RichText::new(format!("{} notes · tempo from the Song bar's speed ({:.0}%)", song.notes_per_ch[parts[0] as usize], self.p.song_speed.get() * 100.0)).size(12.0).color(text_dim()));
                    } else {
                        ui.label(RichText::new("Your part:").color(YOURS));
                        ui.horizontal_wrapped(|ui| {
                            for ch in parts {
                                let prog = song.programs[ch as usize];
                                let inst = if prog >= 0 { self.presets.iter().find(|p| p.bank == 0 && p.prog == prog as i32).map(|p| p.name.clone()).unwrap_or(format!("Prog {}", prog + 1)) } else { "Piano".into() };
                                if chip(ui, self.song_user & (1 << ch) != 0, &format!("{} · {} ({})", ch + 1, inst, song.notes_per_ch[ch as usize])) {
                                    self.song_user ^= 1 << ch;
                                    self.p.song_user.store(self.song_user, Relaxed);
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Speed").color(text_dim()));
                        ui.spacing_mut().slider_width = 260.0;
                        let mut sp = self.p.song_speed.get() * 100.0;
                        if ui.add(egui::Slider::new(&mut sp, 25.0..=150.0).step_by(5.0).suffix(" %")).changed() {
                            self.p.song_speed.set(sp / 100.0);
                        }
                    });
                    ui.add_space(8.0);
                    let can = self.song_user != 0;
                    let r = ui.add_enabled(can, egui::Button::new(RichText::new("▶  PLAY AS GAME").strong().size(14.0).color(Color32::BLACK)).fill(GOOD).corner_radius(8.0).min_size(Vec2::new(170.0, 40.0)));
                    if r.clicked() {
                        self.start_game(true);
                    }
                    if !can {
                        ui.label(RichText::new("Pick your part first").color(BAD));
                    }
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("🎻 Pick a tune…").clicked() {
                    self.tunes_open = true;
                }
                ui.label(RichText::new("or drop a .mid / .abc on the window").size(12.0).color(text_dim()));
            });
        });
    }

    fn menu_bests(&mut self, ui: &mut egui::Ui) {
        if self.s.g_best.is_empty() {
            return;
        }
        panel_card(ui, "PERSONAL BESTS", |ui| {
            let mut v: Vec<(&String, &(f64, String))> = self.s.g_best.iter().collect();
            v.sort_by(|a, b| b.1 .0.partial_cmp(&a.1 .0).unwrap_or(std::cmp::Ordering::Equal));
            egui::Grid::new("bests").num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
                for (k, (acc, g)) in v.into_iter().take(10) {
                    ui.label(RichText::new(g).strong().color(grade_col(g)));
                    ui.label(RichText::new(format!("{:.1}%", acc * 100.0)).monospace());
                    ui.label(RichText::new(k).size(12.0).color(text_dim()));
                    ui.end_row();
                }
            });
        });
    }

    fn game_play(&mut self, ui: &mut egui::Ui, rect: Rect) {
        // HUD strip
        let hud = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 54.0));
        let field = Rect::from_min_max(Pos2::new(rect.left(), hud.bottom() + 4.0), rect.max);
        let Some(g) = self.game.as_ref() else { return };
        let painter = ui.painter_at(hud);
        painter.rect_filled(hud, 10.0, card_bg());
        painter.text(hud.left_center() + Vec2::new(16.0, -9.0), Align2::LEFT_CENTER, "🎮 GAME", FontId::proportional(13.0), YOURS);
        painter.text(hud.left_center() + Vec2::new(16.0, 10.0), Align2::LEFT_CENTER, &g.name, FontId::proportional(14.0), fg());
        let mult = 1 + (g.combo / 25).min(3);
        let mid = hud.center();
        painter.text(mid + Vec2::new(-90.0, 0.0), Align2::CENTER_CENTER, format!("{:07}", g.score), FontId::monospace(28.0), fg());
        painter.text(mid + Vec2::new(40.0, -9.0), Align2::LEFT_CENTER, format!("{:.1}%", g.accuracy() * 100.0), FontId::proportional(16.0), fg());
        painter.text(mid + Vec2::new(40.0, 11.0), Align2::LEFT_CENTER, format!("combo {}  ·  ×{}", g.combo, mult), FontId::proportional(12.0), text_dim());
        // judgement tally
        let tally = [(game::J_PERFECT, "P", YOURS), (game::J_GREAT, "G", GOOD), (game::J_GOOD, "g", ARP), (game::J_MISS, "✗", BAD)];
        for (i, (j, l, c)) in tally.iter().enumerate() {
            let p = mid + Vec2::new(190.0 + i as f32 * 52.0, 0.0);
            painter.text(p, Align2::LEFT_CENTER, format!("{l} {}", g.counts[*j as usize]), FontId::proportional(14.0), *c);
        }
        // progress
        let prog = (self.p.song_pos.get() / g.chart.dur.max(0.1)).clamp(0.0, 1.0) as f32;
        let pr = Rect::from_min_size(Pos2::new(hud.left() + 12.0, hud.bottom() - 6.0), Vec2::new((hud.width() - 24.0) * prog, 3.0));
        painter.rect_filled(Rect::from_min_size(pr.min, Vec2::new(hud.width() - 24.0, 3.0)), 1.0, gray(50));
        painter.rect_filled(pr, 1.0, YOURS);
        // buttons
        let mut stop = false;
        let mut restart = false;
        let br = Rect::from_min_size(Pos2::new(hud.right() - 190.0, hud.top() + 11.0), Vec2::new(180.0, 32.0));
        ui.scope_builder(UiBuilder::new().max_rect(br), |ui| {
            ui.horizontal(|ui| {
                restart = ui.button(RichText::new("↻ Restart").size(14.0)).clicked();
                stop = ui.button(RichText::new("■ Menu").size(14.0)).on_hover_text("Esc").clicked();
            });
        });
        // the playfield: notes falling onto your keys
        self.waterfall(ui, field);
        // count-in
        if let Some(g) = self.game.as_ref() {
            let pos = self.p.song_pos.get();
            let speed = self.p.song_speed.get().max(0.05) as f64;
            let first = g.chart.notes.iter().find(|n| g.mask & (1 << n.ch) != 0).map(|n| n.start);
            if let Some(first) = first {
                let left = (first - pos) / speed;
                if left > 0.05 && pos < first {
                    let beat = if g.name.ends_with("· song") { 0.6 } else { 60.0 / self.s.g_bpm.max(30.0) as f64 };
                    let n = (left / beat).ceil() as i32;
                    let p = ui.painter_at(field);
                    let frac = ((left / beat).fract()) as f32;
                    let size = 90.0 + 40.0 * frac;
                    p.text(field.center() - Vec2::new(0.0, 40.0), Align2::CENTER_CENTER, if n <= 4 { n.to_string() } else { "get ready".into() }, FontId::proportional(if n <= 4 { size } else { 40.0 }), YOURS.gamma_multiply(0.35 + 0.6 * frac));
                }
            }
        }
        if stop {
            let _ = self.tx.send(Cmd::SongPlay(false));
            self.game = None;
        } else if restart {
            let from_song = self.game.as_ref().map(|g| g.name.ends_with("· song")).unwrap_or(false);
            self.start_game(from_song);
        }
    }

    fn game_results(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let Some(g) = self.game.as_ref() else { return };
        let (grade, acc, score, maxc, counts, wrong, total, name) = (g.grade(), g.accuracy(), g.score, g.max_combo, g.counts, g.wrong, g.total, g.name.clone());
        let best = self.s.g_best.get(&name).cloned();
        let new_best = best.as_ref().map(|b| (b.0 - acc).abs() < 1e-9).unwrap_or(false);
        let mut again = false;
        let mut faster = false;
        let mut menu = false;
        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect));
        ui.vertical_centered(|ui| {
            ui.add_space(rect.height() * 0.06);
            ui.label(RichText::new(grade).size(120.0).strong().color(grade_col(grade)));
            ui.label(RichText::new(format!("{:.1}%", acc * 100.0)).size(34.0));
            if new_best {
                ui.label(RichText::new("★ NEW PERSONAL BEST").size(16.0).strong().color(YOURS));
            } else if let Some((b, bg)) = &best {
                ui.label(RichText::new(format!("Personal best: {bg} · {:.1}%", b * 100.0)).size(14.0).color(text_dim()));
            }
            ui.label(RichText::new(&name).size(13.0).color(text_dim()));
            ui.add_space(16.0);
            egui::Frame::new().fill(card_bg()).corner_radius(14.0).inner_margin(egui::Margin::symmetric(28, 16)).show(ui, |ui| {
                egui::Grid::new("res2").num_columns(2).spacing([48.0, 6.0]).show(ui, |ui| {
                    for (lbl, v, c) in [("Perfect", counts[1], YOURS), ("Great", counts[2], GOOD), ("Good", counts[3], ARP), ("Miss", counts[4], BAD), ("Wrong notes", wrong, BAD)] {
                        ui.label(RichText::new(lbl).size(16.0).color(c));
                        ui.label(RichText::new(v.to_string()).size(16.0).monospace());
                        ui.end_row();
                    }
                    ui.label(RichText::new("Max combo").size(16.0));
                    ui.label(RichText::new(format!("{maxc} / {total}")).size(16.0).monospace());
                    ui.end_row();
                    ui.label(RichText::new("Score").size(16.0));
                    ui.label(RichText::new(score.to_string()).size(16.0).monospace());
                    ui.end_row();
                });
            });
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                let w = 3.0 * 150.0 + 2.0 * 8.0;
                ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
                again = big_toggle(ui, true, "↻  Again", GOOD, Vec2::new(150.0, 40.0)).clicked();
                let label = if name.ends_with("· song") { "⏩  Again, +5%" } else { "⏩  Again, +5 bpm" };
                faster = big_toggle(ui, true, label, YOURS, Vec2::new(150.0, 40.0)).clicked();
                menu = big_toggle(ui, false, "Menu", GOOD, Vec2::new(150.0, 40.0)).clicked();
            });
        });
        let from_song = name.ends_with("· song");
        if again || faster {
            if faster {
                if from_song {
                    let sp = (self.p.song_speed.get() + 0.05).min(2.0);
                    self.p.song_speed.set(sp);
                } else {
                    self.s.g_bpm = (self.s.g_bpm + 5.0).min(240.0);
                }
            }
            self.start_game(from_song);
        } else if menu {
            self.game = None;
        }
    }
}
