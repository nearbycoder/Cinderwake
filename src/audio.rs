use crate::game::{Game, Screen, Sfx};
use crate::world::Biome;
use macroquad::audio::*;

/// One looping score per part of a run. Order matches the files in `Audio::new`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    /// Title, Keeper, death, and victory.
    Hearth,
    Aqueduct,
    Conservatory,
    Foundry,
    Crown,
}
impl Track {
    pub const ALL: [Self; 5] = [
        Self::Hearth,
        Self::Aqueduct,
        Self::Conservatory,
        Self::Foundry,
        Self::Crown,
    ];
    /// The score for what's on screen. Menus over a run keep its biome's music.
    pub fn for_game(g: &Game) -> Self {
        let resting = match g.screen {
            Screen::Title | Screen::Camp | Screen::Dead | Screen::Victory => true,
            Screen::Options | Screen::Controls => g.options_from == Screen::Title,
            _ => false,
        };
        if resting {
            return Self::Hearth;
        }
        match g.level.biome {
            Biome::Aqueduct => Self::Aqueduct,
            Biome::Garden => Self::Conservatory,
            Biome::Foundry => Self::Foundry,
            Biome::Crown => Self::Crown,
        }
    }
}

/// Per-track levels: the wanted track rises to full over `SECONDS` while the
/// others fall, on an equal-power curve so the mix doesn't dip mid-fade.
#[derive(Clone, Debug, PartialEq)]
pub struct Crossfade {
    pub levels: [f32; Track::ALL.len()],
}
impl Crossfade {
    pub const SECONDS: f32 = 1.5;
    pub fn new(start: Track) -> Self {
        let mut levels = [0.; Track::ALL.len()];
        levels[start as usize] = 1.;
        Self { levels }
    }
    pub fn step(&mut self, want: Track, dt: f32) {
        let delta = dt / Self::SECONDS;
        for (i, level) in self.levels.iter_mut().enumerate() {
            *level = if i == want as usize {
                (*level + delta).min(1.)
            } else {
                (*level - delta).max(0.)
            };
        }
    }
    /// Volume multiplier for a track.
    pub fn gain(&self, track: Track) -> f32 {
        (self.levels[track as usize] * std::f32::consts::FRAC_PI_2).sin()
    }
}

/// The embedded file for each cue. The match is exhaustive, so a new cue
/// can't compile without a file.
fn effect_file(cue: Sfx) -> &'static [u8] {
    match cue {
        Sfx::Slash => include_bytes!("../assets/slash.wav"),
        Sfx::Hit => include_bytes!("../assets/hit.wav"),
        Sfx::Jump => include_bytes!("../assets/jump.wav"),
        Sfx::Dodge => include_bytes!("../assets/dodge.wav"),
        Sfx::Parry => include_bytes!("../assets/parry.wav"),
        Sfx::Loot => include_bytes!("../assets/loot.wav"),
        Sfx::Hurt => include_bytes!("../assets/hurt.wav"),
        Sfx::Explosion => include_bytes!("../assets/explosion.wav"),
        Sfx::Heal => include_bytes!("../assets/heal.wav"),
        Sfx::Kill => include_bytes!("../assets/kill.wav"),
        Sfx::Bolt => include_bytes!("../assets/bolt.wav"),
        Sfx::Throw => include_bytes!("../assets/throw.wav"),
        Sfx::Bank => include_bytes!("../assets/bank.wav"),
        Sfx::Select => include_bytes!("../assets/select.wav"),
        Sfx::Deny => include_bytes!("../assets/deny.wav"),
        Sfx::Finisher => include_bytes!("../assets/finisher.wav"),
        Sfx::Tell => include_bytes!("../assets/tell.wav"),
    }
}

pub struct Audio {
    sounds: Vec<Option<Sound>>,
    music: Vec<Option<Sound>>,
    fade: Crossfade,
    /// Volume last sent to each track, or `None` while it isn't playing.
    playing: [Option<f32>; Track::ALL.len()],
}
impl Audio {
    pub async fn new() -> Self {
        let mut sounds = vec![];
        for cue in Sfx::ALL {
            sounds.push(load_sound_from_bytes(effect_file(cue)).await.ok());
        }
        let scores: [&[u8]; Track::ALL.len()] = [
            include_bytes!("../assets/music/hearth.wav"),
            include_bytes!("../assets/music/aqueduct.wav"),
            include_bytes!("../assets/music/conservatory.wav"),
            include_bytes!("../assets/music/foundry.wav"),
            include_bytes!("../assets/music/crown.wav"),
        ];
        let mut music = vec![];
        for b in scores {
            music.push(load_sound_from_bytes(b).await.ok());
        }
        Self {
            sounds,
            music,
            fade: Crossfade::new(Track::Hearth),
            playing: [None; Track::ALL.len()],
        }
    }
    /// Gains already include mute; zero silences a channel. `dt` is real time,
    /// so music keeps crossfading while menus freeze the world.
    pub fn update(
        &mut self,
        events: &mut Vec<Sfx>,
        want: Track,
        dt: f32,
        music_gain: f32,
        effects_gain: f32,
    ) {
        self.fade.step(want, dt);
        for track in Track::ALL {
            let i = track as usize;
            let Some(sound) = &self.music[i] else {
                continue;
            };
            let volume = self.fade.gain(track) * music_gain;
            match self.playing[i] {
                None if self.fade.levels[i] > 0. => {
                    // A track that fades back in starts again from the top.
                    play_sound(
                        sound,
                        PlaySoundParams {
                            looped: true,
                            volume,
                        },
                    );
                    self.playing[i] = Some(volume);
                }
                Some(_) if self.fade.levels[i] <= 0. => {
                    stop_sound(sound);
                    self.playing[i] = None;
                }
                Some(sent) if (sent - volume).abs() > 0.002 => {
                    set_sound_volume(sound, volume);
                    self.playing[i] = Some(volume);
                }
                _ => {}
            }
        }
        for e in events.drain(..) {
            if effects_gain > 0. {
                if let Some(s) = &self.sounds[e as usize] {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: false,
                            volume: effects_gain,
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::Save;

    /// Practice games never write progress, so tests leave the data folder alone.
    fn practice() -> Game {
        let mut g = Game::new(7, Save::default());
        g.practice = true;
        g
    }

    #[test]
    fn each_scene_has_its_score() {
        let mut g = practice();
        assert_eq!(Track::for_game(&g), Track::Hearth, "title");
        g.start();
        assert_eq!(Track::for_game(&g), Track::Aqueduct);
        g.screen = Screen::Paused;
        assert_eq!(
            Track::for_game(&g),
            Track::Aqueduct,
            "pause keeps the biome"
        );
        g.open_options();
        assert_eq!(Track::for_game(&g), Track::Aqueduct, "options over a run");
        g.close_options();
        g.screen = Screen::Camp;
        assert_eq!(Track::for_game(&g), Track::Hearth, "the Keeper");
        g.route = 1;
        g.travel();
        assert_eq!(Track::for_game(&g), Track::Foundry);
        g.screen = Screen::Camp;
        g.travel();
        assert_eq!(Track::for_game(&g), Track::Crown);
        g.screen = Screen::Dead;
        assert_eq!(Track::for_game(&g), Track::Hearth);
        let mut g = practice();
        g.start();
        g.screen = Screen::Camp;
        g.route = 0;
        g.travel();
        assert_eq!(Track::for_game(&g), Track::Conservatory);
        let mut g = practice();
        g.open_options();
        assert_eq!(Track::for_game(&g), Track::Hearth, "options over the title");
    }

    #[test]
    fn every_cue_loads_its_own_valid_file_in_order() {
        // `Audio` indexes its sounds by `cue as usize`.
        for (i, cue) in Sfx::ALL.into_iter().enumerate() {
            assert_eq!(cue as usize, i, "{cue:?} is out of order");
            let file = effect_file(cue);
            assert_eq!(&file[..4], b"RIFF", "{cue:?} is a WAV file");
            assert!(file.len() > 1000, "{cue:?} has audio");
        }
        let mut files: Vec<_> = Sfx::ALL.map(effect_file).to_vec();
        files.sort();
        files.dedup();
        assert_eq!(files.len(), Sfx::ALL.len(), "each cue has a distinct file");
    }

    #[test]
    fn crossfades_take_their_time_without_a_dip() {
        let mut fade = Crossfade::new(Track::Hearth);
        assert_eq!(fade.gain(Track::Hearth), 1.);
        let step = 1. / 60.;
        let mut elapsed = 0.;
        while fade.levels[Track::Hearth as usize] > 0. {
            fade.step(Track::Foundry, step);
            elapsed += step;
            let power = fade.gain(Track::Hearth).powi(2) + fade.gain(Track::Foundry).powi(2);
            assert!((power - 1.).abs() < 1e-3, "equal power mid-fade: {power}");
        }
        assert!((elapsed - Crossfade::SECONDS).abs() < 0.05, "{elapsed}");
        assert_eq!(fade.gain(Track::Foundry), 1.);
        // Changing scenes mid-fade reverses smoothly, never beyond full or silent.
        fade.step(Track::Crown, 0.5);
        fade.step(Track::Foundry, 0.2);
        assert!(fade.levels.iter().all(|l| (0. ..=1.).contains(l)));
        assert!(fade.levels[Track::Foundry as usize] > fade.levels[Track::Crown as usize]);
    }
}
