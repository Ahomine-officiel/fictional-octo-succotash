//! Audio stub (v1: muet).
//!
//! Architecture prête pour rodio : chaque événement de gameplay appelle
//! `Audio::play()`. En v2, brancher rodio ici :
//!   - charger les WAV/OGG depuis `assets/audio/sfx/` et `assets/audio/music/`
//!   - OutputSink → Sink par canal (sfx / musique)
//!   - crossfade musique de combat vs exploration

#[derive(Clone, Copy, Debug)]
pub enum Sfx {
    MeleeSwing,
    MeleeHit,
    Crit,
    BowShoot,
    ArrowHit,
    Roll,
    Potion,
    Chest,
    Emerald,
    EnemyDie,
    PlayerHurt,
    LevelUp,
    Artifact,
    BossRoar,
    Portal,
    UiClick,
    UiOpen,
}

#[derive(Clone, Copy, Debug)]
pub enum Music {
    Menu,
    Exploration,
    Combat,
    Boss,
    Victory,
}

pub struct Audio;

impl Audio {
    pub fn new() -> Audio {
        // v2: init rodio output stream + charge les assets
        Audio
    }

    pub fn play(&self, sfx: Sfx) {
        if !crate::consts::AUDIO_ENABLED {
            return;
        }
        let _ = sfx; // v2: sink.append(Decoder::new(file_for(sfx)))
    }

    pub fn set_music(&mut self, music: Music) {
        if !crate::consts::AUDIO_ENABLED {
            return;
        }
        let _ = music; // v2: crossfade
    }

    pub fn update(&mut self) {}
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}
