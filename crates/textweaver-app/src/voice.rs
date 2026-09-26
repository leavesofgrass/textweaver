//! Rate, pitch, volume, speed presets, and display toggles. Every change is
//! announced (Star showed rate changes only visually) and marks the settings
//! for saving on quit.

use textweaver_a11y::{Announcement, Verbosity};
use textweaver_core::{Pitch, Rate, Volume};
use textweaver_speech::Earcon;

use crate::app::App;

/// What a chosen voice says as its sample.
pub const VOICE_SAMPLE: &str = "The quick brown fox jumps over the lazy dog.";

/// A voice as listed: its name, then its languages and tags ("Microsoft
/// Zira, en-US, OneCore"), "favourite" for one in
/// `speech.favorite_voices`, and "current" for the one in use.
fn voice_label(v: &textweaver_speech::Voice, favourite: bool, current: bool) -> String {
    let mut parts = vec![v.name.clone()];
    if let Some(l) = v.languages.first() {
        parts.push(l.clone());
    }
    parts.extend(v.tags.iter().cloned());
    if favourite {
        parts.push("favourite".into());
    }
    if current {
        parts.push("current".into());
    }
    parts.join(", ")
}

/// Where `(id, name)` is in the favourites (matched by id, or by name
/// ignoring case), if it is one.
fn favourite_rank(favourites: &[String], id: &str, name: &str) -> Option<usize> {
    favourites
        .iter()
        .position(|f| f == id || f.eq_ignore_ascii_case(name))
}

impl App {
    /// Choose voice (Alt+V): lists the engine's voices; Enter selects one
    /// and speaks a sample. The list was made once when the engine started
    /// and is read without waiting; while it is still being made, this
    /// says so and the list opens when it arrives ([`voices_tick`](Self::voices_tick)).
    pub(crate) fn choose_voice(&mut self) -> Vec<crate::command::Effect> {
        use crate::command::Effect;
        let voices = match self.speech.voice_list() {
            textweaver_speech::VoiceList::Ready(v) => v,
            textweaver_speech::VoiceList::Loading => {
                self.voices_pending = true;
                // The frontend is woken when they arrive (crate::wake).
                let wake = self.waker_slot();
                self.speech.voice_cache().on_ready(move || wake.wake());
                self.tell("The voices are still loading. The list opens when they are ready.");
                return vec![Effect::Redraw];
            }
            textweaver_speech::VoiceList::Failed(e) => {
                self.error(&format!("Could not list the voices: {e}."));
                return vec![Effect::Redraw];
            }
        };
        if voices.is_empty() {
            self.tell("This speech engine has no voices to choose from.");
            return vec![Effect::Redraw];
        }
        let mut voices = voices;
        // Favourites first, in the order they were added; the rest keep
        // the engine's order.
        let favourites = self.settings.speech.favorite_voices.clone();
        voices.sort_by_key(|v| favourite_rank(&favourites, &v.id, &v.name).unwrap_or(usize::MAX));
        let n = voices.len();
        let items = self.voice_items(&voices);
        self.voice_list = voices;
        self.list = Some(crate::app::ListKind::Voices(
            self.voice_list
                .iter()
                .map(|v| (v.id.clone(), v.name.clone()))
                .collect(),
        ));
        self.tell(&format!(
            "Voices, {n} {}, favourites first. Enter chooses one and speaks a sample, Space adds or removes a favourite, Escape cancels.",
            if n == 1 { "voice" } else { "voices" }
        ));
        vec![Effect::ShowList {
            title: "Choose a voice".into(),
            items,
        }]
    }

    /// Opens the voice list asked for while the voices were loading, once
    /// they arrive (from [`App::tick`]); if something else is open by
    /// then, says they are ready instead.
    pub(crate) fn voices_tick(&mut self) -> Vec<crate::command::Effect> {
        if !self.voices_pending || self.speech.voice_list().is_loading() {
            return Vec::new();
        }
        self.voices_pending = false;
        if self.list.is_some() || self.mode.is_prompt() || self.confirmation_pending() {
            let keys =
                crate::help::chords_text(&self.keymap, textweaver_keymap::ActionId::ChooseVoice);
            self.tell(&format!("The voices are ready. {keys} lists them."));
            return vec![crate::command::Effect::Redraw];
        }
        self.choose_voice()
    }

    fn voice_items(&self, voices: &[textweaver_speech::Voice]) -> Vec<String> {
        let current = self.settings.speech.voice.clone();
        let favourites = &self.settings.speech.favorite_voices;
        voices
            .iter()
            .map(|v| {
                let is_current = current
                    .as_deref()
                    .is_some_and(|c| c == v.id || c.eq_ignore_ascii_case(&v.name));
                let fav = favourite_rank(favourites, &v.id, &v.name).is_some();
                voice_label(v, fav, is_current)
            })
            .collect()
    }

    /// Space in the voice list: adds voice `n` to `speech.favorite_voices`,
    /// or removes it, saved at once. The list stays open, in the same
    /// order, with the item relabelled.
    pub(crate) fn toggle_favourite_voice(&mut self, n: usize) -> Vec<crate::command::Effect> {
        use crate::command::Effect;
        let Some(v) = self.voice_list.get(n).cloned() else {
            return vec![Effect::Redraw];
        };
        let favs = &mut self.settings.speech.favorite_voices;
        let msg = match favourite_rank(favs, &v.id, &v.name) {
            Some(i) => {
                favs.remove(i);
                format!("{} removed from favourites.", v.name)
            }
            None => {
                favs.push(v.id.clone());
                format!("{} added to favourites.", v.name)
            }
        };
        self.settings_dirty = true;
        self.tell(&msg);
        let items = self.voice_items(&self.voice_list);
        vec![Effect::ShowList {
            title: "Choose a voice".into(),
            items,
        }]
    }

    /// Uses voice `id` from now on (saved in the settings) and speaks a
    /// sample with it.
    pub(crate) fn select_voice(&mut self, id: &str, name: &str) {
        self.stop_speech();
        self.settings.speech.voice = Some(id.to_owned());
        self.settings_dirty = true;
        self.speech.set_voice(Some(id.to_owned()));
        self.tell(&format!("Voice {name}."));
        self.speech
            .say(VOICE_SAMPLE, textweaver_speech::SayMode::Queue);
    }

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

    /// F9: single-key shortcuts on or off (`[keyboard] character_keys`), so
    /// dictation and typing never trigger commands. Saved at once.
    pub(crate) fn toggle_character_keys(&mut self) {
        let on = !self.settings.keyboard.character_keys;
        self.settings.keyboard.character_keys = on;
        self.keymap.set_character_keys(on);
        self.settings_dirty = true;
        let a = Announcement::CharacterKeys { on };
        let verbosity = self.settings.speech.verbosity;
        if let Some(text) = a.text(verbosity) {
            self.say_at(&format!("{text}."), Verbosity::Low, a.priority());
        }
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
