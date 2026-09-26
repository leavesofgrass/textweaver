//! Rate, pitch, volume, speed presets, and display toggles. Every change is
//! announced (Star showed rate changes only visually) and marks the settings
//! for saving on quit.

use textweaver_core::{Pitch, Rate, Volume};
use textweaver_speech::Earcon;

use crate::app::App;

impl App {
    /// Sends the voice settings to the speech service.
    pub(crate) fn apply_voice_settings(&mut self) {
        let sp = &self.settings.speech;
        self.speech.set_rate(sp.rate);
        self.speech.set_pitch(sp.pitch);
        self.speech.set_volume(sp.volume);
        if sp.voice.is_some() {
            self.speech.set_voice(sp.voice.clone());
        }
        self.speech.set_punctuation(sp.punctuation);
        self.speech.set_split_caps(sp.split_caps);
    }

    pub(crate) fn set_rate(&mut self, rate: Rate) {
        self.settings.speech.rate = rate;
        self.settings_dirty = true;
        self.speech.set_rate(rate);
    }

    pub(crate) fn change_rate(&mut self, delta: i32) {
        let old = self.settings.speech.rate;
        let new = old.step(delta);
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            self.tell(if delta > 0 {
                "Fastest rate."
            } else {
                "Slowest rate."
            });
            return;
        }
        self.set_rate(new);
        self.tell(&format!("{} words per minute.", new.wpm()));
    }

    pub(crate) fn change_pitch(&mut self, delta: i8) {
        let old = self.settings.speech.pitch;
        let new = old.step(delta);
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            self.tell(if delta > 0 {
                "Highest pitch."
            } else {
                "Lowest pitch."
            });
            return;
        }
        self.settings.speech.pitch = new;
        self.settings_dirty = true;
        self.speech.set_pitch(new);
        self.tell(&pitch_words(new));
    }

    pub(crate) fn change_volume(&mut self, delta: i16) {
        let old = self.settings.speech.volume;
        let new = old.step(delta);
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            self.tell(if delta > 0 {
                "Full volume."
            } else {
                "Volume off."
            });
            return;
        }
        self.settings.speech.volume = new;
        self.settings_dirty = true;
        self.speech.set_volume(new);
        self.tell(&volume_words(new));
    }

    /// Cycles the speed presets from fastest to slowest (Star's order: skim,
    /// normal, study, slow), starting after the preset matching the rate.
    pub(crate) fn cycle_speed_preset(&mut self) {
        let mut presets: Vec<(String, u16)> = self
            .settings
            .speech
            .speed_presets
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        if presets.is_empty() {
            self.tell("No speed presets.");
            return;
        }
        presets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let wpm = self.settings.speech.rate.wpm();
        let next = match presets.iter().position(|(_, w)| *w == wpm) {
            Some(i) => (i + 1) % presets.len(),
            None => presets.iter().position(|(_, w)| *w < wpm).unwrap_or(0),
        };
        let (name, w) = presets[next].clone();
        self.set_rate(Rate::Wpm(w).clamped());
        self.tell(&format!(
            "{} speed, {} words per minute.",
            capitalize(&name),
            Rate::Wpm(w).clamped().wpm()
        ));
    }

    pub(crate) fn next_theme(&mut self) {
        let current = self.settings.display.theme.as_str();
        let i = Self::THEMES.iter().position(|t| *t == current);
        let next = Self::THEMES[i.map_or(0, |i| (i + 1) % Self::THEMES.len())];
        self.settings.display.theme = next.to_owned();
        self.settings_dirty = true;
        self.tell(&format!("Theme {}.", next.replace('-', " ")));
    }

    pub(crate) fn toggle_line_numbers(&mut self) {
        let on = !self.settings.display.show_line_numbers;
        self.settings.display.show_line_numbers = on;
        self.settings_dirty = true;
        self.tell(if on {
            "Line numbers on."
        } else {
            "Line numbers off."
        });
    }
}

/// "Pitch plus 2", "Pitch minus 1", "Normal pitch".
fn pitch_words(p: Pitch) -> String {
    match p.semitones() {
        0 => "Normal pitch.".into(),
        s if s > 0 => format!("Pitch plus {s}."),
        s => format!("Pitch minus {}.", s.unsigned_abs()),
    }
}

fn volume_words(v: Volume) -> String {
    format!("Volume {} percent.", v.percent())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_words_read_well() {
        assert_eq!(pitch_words(Pitch::Semitones(-2)), "Pitch minus 2.");
        assert_eq!(pitch_words(Pitch::Semitones(0)), "Normal pitch.");
        assert_eq!(volume_words(Volume::new(90)), "Volume 90 percent.");
    }
}
