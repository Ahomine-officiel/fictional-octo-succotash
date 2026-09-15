//! Audio procédural : TOUS les sons sont synthétisés au démarrage (aucun
//! fichier asset) et joués via rodio. Une seule source de vérité : le global
//! `AUDIO` (OnceLock<Mutex<AudioState>>) — `sfx()` est appelable de partout
//! (gameplay) sans threading du handle.
//!
//! Recette par SFX : oscillateurs (sine / saw / bruit) + enveloppes ADSR
//! simplifiées + petits arpèges. Musique = boucles générées (pads + arpèges
//! + percussions) par ambiance (menu / exploration / combat / boss / victoire).

use rodio::source::Source;
use rodio::{OutputStream, Sink};
use std::sync::Mutex;
use std::time::Duration;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Music {
    Menu,
    Exploration,
    Combat,
    Boss,
    Victory,
}

const SR: u32 = 22050; // suffisant pour du chiptune, moitié moins de données

// ----------------------------------------------------------------------
// Petit moteur de synthé
// ----------------------------------------------------------------------

/// Oscillateur : 0=sine, 1=square, 2=saw, 3=noise
fn osc(kind: u8, phase: f32) -> f32 {
    match kind {
        0 => (phase * std::f32::consts::TAU).sin(),
        1 => (if (phase * std::f32::consts::TAU).sin() > 0.0 { 1.0 } else { -1.0 }),
        2 => phase.fract() * 2.0 - 1.0,
        _ => ((phase * 12.9898).fract() * 43758.5453).fract() * 2.0 - 1.0,
    }
}

fn midi(n: f32) -> f32 {
    2.0f32.powf((n - 69.0) / 12.0) * 440.0
}

/// Note : (forme, note midi, début, durée, volume, glide midi final Option)
struct Note(u8, f32, f32, f32, f32, Option<f32>);

fn render_notes(len: f32, notes: &[Note]) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    let mut out = vec![0.0f32; n];
    for Note(kind, f0, t0, dur, vol, f1) in notes {
        let start = (*t0 * SR as f32) as usize;
        let nlen = (*dur * SR as f32) as usize;
        for i in 0..nlen {
            let si = start + i;
            if si >= n {
                break;
            }
            let t = i as f32 / SR as f32;
            let p = i as f32 / nlen as f32;
            // enveloppe : attaque 5 ms, decay exponentiel
            let env = if t < 0.005 {
                t / 0.005
            } else {
                (1.0 - p).powf(if *kind == 3 { 1.6 } else { 2.2 })
            };
            let f = midi(match f1 {
                Some(f2) => f0 + (f2 - f0) * p,
                None => *f0,
            });
            out[si] += osc(*kind, f * t) * env * vol;
        }
    }
    out
}

fn to_i16(v: &[f32], gain: f32) -> rodio::buffer::SamplesBuffer<i16> {
    let samples: Vec<i16> = v
        .iter()
        .map(|x| (x.clamp(-1.0, 1.0) * gain * 32767.0) as i16)
        .collect();
    rodio::buffer::SamplesBuffer::new(1, SR, samples)
}

fn mix(a: Vec<f32>, b: Vec<f32>) -> Vec<f32> {
    let n = a.len().max(b.len());
    let mut out = vec![0.0f32; n];
    for (i, o) in out.iter_mut().enumerate() {
        let x = a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0);
        *o = x * 0.8;
    }
    out
}

// ----------------------------------------------------------------------
// Génération des SFX
// ----------------------------------------------------------------------

fn make_sfx(s: Sfx) -> rodio::buffer::SamplesBuffer<i16> {
    use Sfx::*;
    let v = match s {
        MeleeSwing => {
            // whoosh : bruit modelé par un balayage d'amplitude
            let n = (0.16 * SR as f32) as usize;
            (0..n)
                .map(|i| {
                    let p = i as f32 / n as f32;
                    let env = (p * std::f32::consts::PI).sin();
                    osc(3, i as f32 * 0.05) * env * 0.5
                })
                .collect()
        }
        MeleeHit => mix(
            render_notes(0.14, &[Note(0, 45.0, 0.0, 0.14, 0.9, Some(31.0))]),
            render_notes(0.14, &[Note(3, 60.0, 0.0, 0.06, 0.5, None)]),
        ),
        Crit => mix(
            render_notes(0.22, &[Note(0, 45.0, 0.0, 0.14, 0.8, Some(31.0))]),
            render_notes(0.22, &[Note(0, 98.0, 0.02, 0.2, 0.5, None)]),
        ),
        BowShoot => render_notes(0.16, &[Note(2, 74.0, 0.0, 0.14, 0.6, Some(52.0))]),
        ArrowHit => mix(
            render_notes(0.08, &[Note(0, 60.0, 0.0, 0.08, 0.7, Some(45.0))]),
            render_notes(0.08, &[Note(3, 80.0, 0.0, 0.03, 0.4, None)]),
        ),
        Roll => {
            // longue whoosh montante-descendante
            let n = (0.3 * SR as f32) as usize;
            (0..n)
                .map(|i| {
                    let p = i as f32 / n as f32;
                    let env = (p * std::f32::consts::PI).sin();
                    osc(3, i as f32 * 0.02) * env * 0.55
                })
                .collect()
        }
        Potion => render_notes(
            0.35,
            &[Note(0, 62.0, 0.0, 0.32, 0.5, Some(74.0))],
        ),
        Chest => mix(
            render_notes(0.2, &[Note(2, 40.0, 0.0, 0.18, 0.4, Some(36.0))]),
            render_notes(
                0.6,
                &[
                    Note(0, 76.0, 0.12, 0.25, 0.45, None), // E5
                    Note(0, 83.0, 0.24, 0.3, 0.45, None),  // B5
                ],
            ),
        ),
        Emerald => render_notes(
            0.18,
            &[Note(0, 93.0, 0.0, 0.16, 0.5, Some(100.0))],
        ),
        EnemyDie => mix(
            render_notes(0.35, &[Note(2, 57.0, 0.0, 0.3, 0.6, Some(38.0))]),
            render_notes(0.35, &[Note(3, 50.0, 0.0, 0.2, 0.3, None)]),
        ),
        PlayerHurt => mix(
            render_notes(0.2, &[Note(1, 43.0, 0.0, 0.16, 0.7, Some(38.0))]),
            render_notes(0.2, &[Note(3, 60.0, 0.0, 0.08, 0.4, None)]),
        ),
        LevelUp => render_notes(
            0.7,
            &[
                Note(0, 72.0, 0.0, 0.18, 0.5, None), // C5
                Note(0, 76.0, 0.12, 0.18, 0.5, None), // E5
                Note(0, 79.0, 0.24, 0.18, 0.5, None), // G5
                Note(0, 84.0, 0.36, 0.32, 0.55, None), // C6
            ],
        ),
        Artifact => render_notes(
            0.5,
            &[
                Note(0, 86.0, 0.0, 0.4, 0.35, Some(93.0)),
                Note(0, 93.0, 0.06, 0.4, 0.3, None),
                Note(0, 98.0, 0.12, 0.4, 0.3, None),
            ],
        ),
        BossRoar => mix(
            render_notes(0.9, &[Note(2, 33.0, 0.0, 0.85, 0.9, Some(28.0))]),
            render_notes(0.9, &[Note(3, 30.0, 0.0, 0.6, 0.5, None)]),
        ),
        Portal => mix(
            render_notes(0.8, &[Note(0, 60.0, 0.0, 0.75, 0.5, Some(88.0))]),
            render_notes(
                0.8,
                &[Note(0, 93.0, 0.3, 0.45, 0.3, Some(105.0))],
            ),
        ),
        UiClick => render_notes(0.05, &[Note(0, 88.0, 0.0, 0.045, 0.4, None)]),
        UiOpen => render_notes(
            0.12,
            &[
                Note(0, 76.0, 0.0, 0.06, 0.4, None),
                Note(0, 83.0, 0.05, 0.07, 0.4, None),
            ],
        ),
    };
    to_i16(&v, 0.85)
}

// ----------------------------------------------------------------------
// Musiques générées (boucles)
// ----------------------------------------------------------------------

fn make_music(m: Music) -> rodio::buffer::SamplesBuffer<i16> {
    let (len, notes) = match m {
        Music::Menu => {
            // nappe nocturne : accord Cm + cloches éparses
            (
                8.0,
                vec![
                    Note(0, 48.0, 0.0, 7.8, 0.16, None),  // C3
                    Note(0, 51.0, 0.0, 7.8, 0.13, None),  // Eb3
                    Note(0, 55.0, 0.0, 7.8, 0.12, None),  // G3
                    Note(0, 75.0, 1.0, 1.2, 0.18, None),  // bells
                    Note(0, 82.0, 3.0, 1.4, 0.16, None),
                    Note(0, 79.0, 5.5, 1.6, 0.15, None),
                    Note(0, 70.0, 6.6, 1.2, 0.14, None),
                ],
            )
        }
        Music::Exploration => {
            // arpège A mineur sautillant + basse
            (
                4.0,
                vec![
                    Note(0, 45.0, 0.0, 3.8, 0.18, None), // A2 basse
                    Note(0, 69.0, 0.0, 0.4, 0.16, None),
                    Note(0, 72.0, 0.5, 0.4, 0.15, None),
                    Note(0, 76.0, 1.0, 0.4, 0.16, None),
                    Note(0, 72.0, 1.5, 0.4, 0.14, None),
                    Note(0, 69.0, 2.0, 0.4, 0.16, None),
                    Note(0, 76.0, 2.5, 0.4, 0.15, None),
                    Note(0, 81.0, 3.0, 0.4, 0.16, None),
                    Note(0, 76.0, 3.5, 0.4, 0.13, None),
                ],
            )
        }
        Music::Combat => {
            // pompe tendue : basse saw + pulses + toms bruit
            (
                2.4,
                vec![
                    Note(2, 40.0, 0.0, 0.5, 0.35, None),
                    Note(2, 40.0, 0.6, 0.5, 0.3, None),
                    Note(2, 46.0, 1.2, 0.5, 0.35, None),
                    Note(2, 40.0, 1.8, 0.5, 0.3, None),
                    Note(1, 64.0, 0.3, 0.2, 0.14, None),
                    Note(1, 67.0, 0.9, 0.2, 0.13, None),
                    Note(1, 63.0, 1.5, 0.2, 0.14, None),
                    Note(1, 70.0, 2.1, 0.2, 0.13, None),
                    Note(3, 45.0, 0.0, 0.09, 0.5, None),
                    Note(3, 45.0, 1.2, 0.09, 0.5, None),
                ],
            )
        }
        Music::Boss => {
            // drone menaçant + arpège rapide chromatique + gros toms
            (
                2.0,
                vec![
                    Note(2, 31.0, 0.0, 1.9, 0.4, None),  // G1 drone
                    Note(2, 34.0, 0.0, 1.9, 0.2, None),
                    Note(1, 58.0, 0.0, 0.16, 0.2, None),
                    Note(1, 61.0, 0.25, 0.16, 0.2, None),
                    Note(1, 58.0, 0.5, 0.16, 0.2, None),
                    Note(1, 64.0, 0.75, 0.16, 0.2, None),
                    Note(1, 58.0, 1.0, 0.16, 0.2, None),
                    Note(1, 60.0, 1.25, 0.16, 0.2, None),
                    Note(1, 57.0, 1.5, 0.16, 0.2, None),
                    Note(1, 63.0, 1.75, 0.16, 0.2, None),
                    Note(3, 40.0, 0.0, 0.12, 0.6, None),
                    Note(3, 40.0, 1.0, 0.12, 0.6, None),
                ],
            )
        }
        Music::Victory => {
            // fanfare dorée
            (
                4.0,
                vec![
                    Note(0, 72.0, 0.0, 0.3, 0.5, None),
                    Note(0, 72.0, 0.25, 0.3, 0.5, None),
                    Note(0, 72.0, 0.5, 0.5, 0.5, None),
                    Note(0, 76.0, 1.1, 0.35, 0.5, None),
                    Note(0, 79.0, 1.5, 0.6, 0.55, None),
                    Note(0, 84.0, 2.2, 1.2, 0.55, None),
                    Note(0, 79.0, 2.2, 1.2, 0.3, None),
                    Note(0, 76.0, 2.2, 1.2, 0.25, None),
                ],
            )
        }
    };
    to_i16(&render_notes(len, &notes), 0.8)
}

// ----------------------------------------------------------------------
// Thread audio dédié : OutputStream (non-Send) + Sinks vivent ICI, le
// thread principal lui parle via un canal mpsc. Silencieux si aucun
// périphérique (ou MD_MUTE=1).
// ----------------------------------------------------------------------

enum Cmd {
    Play(Sfx),
    Music(Music),
}

static AUDIO_TX: std::sync::OnceLock<Option<std::sync::mpsc::Sender<Cmd>>> =
    std::sync::OnceLock::new();

fn audio_init() -> Option<std::sync::mpsc::Sender<Cmd>> {
    if std::env::var("MD_MUTE").is_ok() {
        log::info!("audio: MD_MUTE — désactivé");
        return None;
    }
    let (tx, rx) = std::sync::mpsc::channel::<Cmd>();
    let ok = std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            // cpal Stream est !Send : il est CRÉÉ et vit entièrement dans CE
            // thread. Tout le matos audio (stream + sinks) reste ici.
            let (stream, handle) = match OutputStream::try_default() {
                Ok(x) => x,
                Err(_) => {
                    log::info!("audio: aucun périphérique de sortie — muet");
                    return;
                }
            };
            let sfx_sink = match Sink::try_new(&handle) {
                Ok(s) => s,
                Err(_) => {
                    let _keep = stream;
                    return;
                }
            };
            let music_sink = match Sink::try_new(&handle) {
                Ok(s) => s,
                Err(_) => {
                    let _keep = stream;
                    return;
                }
            };
            sfx_sink.set_volume(0.55);
            music_sink.set_volume(0.22);
            let mut current: Option<Music> = None;
            log::info!("audio: sortie OK (synthèse procédurale active)");
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Cmd::Play(s) => {
                        // file courte : évite l'effet mitraillette en cas de spam
                        if sfx_sink.len() < 4 {
                            sfx_sink.append(make_sfx(s));
                        }
                    }
                    Cmd::Music(m) => {
                        if current != Some(m) {
                            current = Some(m);
                            music_sink.clear();
                            let src = make_music(m).repeat_infinite();
                            music_sink.append(Box::new(src) as Box<dyn Source<Item = i16> + Send>);
                        }
                    }
                }
            }
            let _keep = stream; // garde la sortie ouverte jusqu'à la fin
        })
        .is_ok();
    if ok {
        Some(tx)
    } else {
        None
    }
}

fn audio_send(cmd: Cmd) {
    if let Some(tx) = AUDIO_TX.get_or_init(audio_init).as_ref() {
        let _ = tx.send(cmd);
    }
}

/// Joue un effet sonore (appelable du gameplay, thread principal).
pub fn sfx(s: Sfx) {
    audio_send(Cmd::Play(s));
}

/// Handle applicatif (une instance dans App) — délègue au thread audio.
pub struct Audio;

impl Audio {
    pub fn new() -> Audio {
        // démarre le thread audio tôt (premier accès au global)
        audio_init();
        Audio
    }

    pub fn play(&self, s: Sfx) {
        sfx(s);
    }

    pub fn set_music(&mut self, music: Music) {
        audio_send(Cmd::Music(music));
    }

    pub fn update(&mut self) {}
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}
