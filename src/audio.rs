use crate::game::Sfx;
use macroquad::audio::*;
pub struct Audio {
    sounds: Vec<Option<Sound>>,
    music: Option<Sound>,
    muted: bool,
}
impl Audio {
    pub async fn new() -> Self {
        let files: [&[u8]; 9] = [
            include_bytes!("../assets/slash.wav"),
            include_bytes!("../assets/hit.wav"),
            include_bytes!("../assets/jump.wav"),
            include_bytes!("../assets/dodge.wav"),
            include_bytes!("../assets/parry.wav"),
            include_bytes!("../assets/loot.wav"),
            include_bytes!("../assets/hurt.wav"),
            include_bytes!("../assets/explosion.wav"),
            include_bytes!("../assets/heal.wav"),
        ];
        let mut sounds = vec![];
        for b in files {
            sounds.push(load_sound_from_bytes(b).await.ok());
        }
        let music = load_sound_from_bytes(include_bytes!("../assets/ambience.wav"))
            .await
            .ok();
        if let Some(s) = &music {
            play_sound(
                s,
                PlaySoundParams {
                    looped: true,
                    volume: 0.5,
                },
            );
        }
        Self {
            sounds,
            music,
            muted: false,
        }
    }
    pub fn update(&mut self, events: &mut Vec<Sfx>, muted: bool) {
        if self.muted != muted {
            if let Some(s) = &self.music {
                set_sound_volume(s, if muted { 0. } else { 0.5 });
            }
            self.muted = muted;
        }
        for e in events.drain(..) {
            if !muted {
                if let Some(s) = &self.sounds[e as usize] {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: false,
                            volume: 0.35,
                        },
                    );
                }
            }
        }
    }
}
