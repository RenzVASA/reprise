//! Le filet de sécurité : Reprise surveille discrètement si tu t'absentes (écran verrouillé,
//! Mac en veille, longue inactivité) pour sauvegarder ton contexte toute seule, et pour te
//! proposer de reprendre quand tu reviens.
//!
//! Pas de bibliothèque système compliquée : on lit `ioreg`, l'outil fourni avec macOS, toutes
//! les 15 secondes. C'est très léger.

use std::process::Command;

/// Ce que le filet de sécurité décide de faire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Prendre une sauvegarde automatique.
    Snapshot,
    /// Tu reviens après une vraie absence (verrouillage ou veille) : proposer de reprendre.
    WelcomeBack { away_ms: u64 },
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Inactivité (clavier, souris) au bout de laquelle on sauvegarde.
    pub idle_snapshot_secs: u64,
    /// Absence minimale pour proposer de reprendre en revenant.
    pub welcome_after_ms: u64,
}

/// Écart entre deux vérifications au-delà duquel on considère que le Mac a dormi.
const SLEEP_GAP_MS: u64 = 120_000;
/// En dessous de cette inactivité, tu es là.
const ACTIVE_SECS: u64 = 60;

#[derive(Debug, Default)]
pub struct Watch {
    last_tick: u64,
    locked: bool,
    /// Déjà sauvegardé pour cette absence.
    snapped: bool,
    away_since: Option<u64>,
    /// L'absence vient d'un verrouillage ou d'une mise en veille (pas juste d'un film regardé).
    real_absence: bool,
}

impl Watch {
    pub fn tick(&mut self, now: u64, locked: Option<bool>, idle_secs: Option<u64>, cfg: &Config) -> Vec<Event> {
        let mut events = Vec::new();
        let locked = locked.unwrap_or(false);
        let idle = idle_secs.unwrap_or(0);

        // Le Mac a dormi entre deux vérifications.
        if self.last_tick > 0 && now.saturating_sub(self.last_tick) > SLEEP_GAP_MS {
            self.away_since.get_or_insert(self.last_tick);
            self.real_absence = true;
        }

        // Début d'absence : verrouillage…
        if locked && !self.locked {
            if !self.snapped {
                events.push(Event::Snapshot);
                self.snapped = true;
            }
            self.away_since.get_or_insert(now);
            self.real_absence = true;
        }
        // … ou longue inactivité.
        if !locked && idle >= cfg.idle_snapshot_secs && !self.snapped {
            events.push(Event::Snapshot);
            self.snapped = true;
            self.away_since.get_or_insert(now.saturating_sub(idle * 1000));
        }

        // Retour.
        if !locked && idle < ACTIVE_SECS {
            if let Some(since) = self.away_since.take() {
                let away_ms = now.saturating_sub(since);
                if self.real_absence && away_ms >= cfg.welcome_after_ms {
                    events.push(Event::WelcomeBack { away_ms });
                }
            }
            self.snapped = false;
            self.real_absence = false;
        }

        self.locked = locked;
        self.last_tick = now;
        events
    }
}

// ─────────────────────────── Lecture de l'état du Mac ───────────────────────────

fn ioreg(args: &[&str]) -> Option<String> {
    let out = Command::new("/usr/sbin/ioreg").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// L'écran est-il verrouillé ? (`None` si on ne sait pas le lire.)
pub fn screen_locked() -> Option<bool> {
    ioreg(&["-n", "Root", "-d1", "-a"]).map(|o| parse_locked(&o))
}

/// Depuis combien de secondes le clavier et la souris n'ont pas bougé.
pub fn idle_seconds() -> Option<u64> {
    ioreg(&["-c", "IOHIDSystem", "-d", "4"]).and_then(|o| parse_idle(&o))
}

pub fn parse_locked(plist: &str) -> bool {
    let key = "<key>CGSSessionScreenIsLocked</key>";
    match plist.find(key) {
        Some(i) => plist[i + key.len()..].trim_start().starts_with("<true/>"),
        None => false,
    }
}

pub fn parse_idle(text: &str) -> Option<u64> {
    let key = "\"HIDIdleTime\" = ";
    let i = text.find(key)? + key.len();
    let digits: String = text[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().ok().map(|ns| ns / 1_000_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: Config = Config { idle_snapshot_secs: 300, welcome_after_ms: 20 * 60_000 };
    const MIN: u64 = 60_000;

    #[test]
    fn lock_then_unlock_after_a_while() {
        let mut w = Watch::default();
        let t0 = 1_000_000_000;
        assert!(w.tick(t0, Some(false), Some(2), &CFG).is_empty());
        assert_eq!(w.tick(t0 + 15_000, Some(true), Some(5), &CFG), vec![Event::Snapshot]);
        // Pendant le verrouillage : rien de plus.
        let mut t = t0 + 30_000;
        while t < t0 + 30 * MIN {
            assert!(w.tick(t, Some(true), Some((t - t0) / 1000), &CFG).is_empty());
            t += 15_000;
        }
        let ev = w.tick(t, Some(false), Some(1), &CFG);
        assert!(matches!(ev.as_slice(), [Event::WelcomeBack { away_ms }] if *away_ms >= 29 * MIN));
    }

    #[test]
    fn short_lock_saves_but_does_not_nag() {
        let mut w = Watch::default();
        let t0 = 1_000_000_000;
        w.tick(t0, Some(false), Some(0), &CFG);
        assert_eq!(w.tick(t0 + 15_000, Some(true), Some(0), &CFG), vec![Event::Snapshot]);
        assert!(w.tick(t0 + 30_000, Some(true), Some(30), &CFG).is_empty());
        assert!(w.tick(t0 + 45_000, Some(false), Some(0), &CFG).is_empty());
    }

    /// Avance minute par minute (vérifications toutes les 15 s), en renvoyant tous les événements.
    fn run(w: &mut Watch, from: u64, to: u64, idle_at: impl Fn(u64) -> u64) -> Vec<Event> {
        let mut out = Vec::new();
        let mut t = from;
        while t <= to {
            out.extend(w.tick(t, Some(false), Some(idle_at(t)), &CFG));
            t += 15_000;
        }
        out
    }

    #[test]
    fn watching_a_film_saves_once_and_never_nags() {
        let mut w = Watch::default();
        let t0 = 1_000_000_000;
        // 40 minutes sans toucher au clavier, puis retour.
        let ev = run(&mut w, t0, t0 + 40 * MIN, |t| (t - t0) / 1000);
        assert_eq!(ev, vec![Event::Snapshot]);
        assert!(w.tick(t0 + 40 * MIN + 15_000, Some(false), Some(3), &CFG).is_empty());
        // Une nouvelle absence peut de nouveau sauvegarder.
        let t1 = t0 + 41 * MIN;
        let ev = run(&mut w, t1, t1 + 6 * MIN, |t| (t - t1) / 1000);
        assert_eq!(ev, vec![Event::Snapshot]);
    }

    #[test]
    fn sleep_is_detected_by_the_time_gap() {
        let mut w = Watch::default();
        let t0 = 1_000_000_000;
        w.tick(t0, Some(false), Some(0), &CFG);
        // Réveil 2 heures plus tard, écran verrouillé, puis déverrouillage.
        assert_eq!(w.tick(t0 + 120 * MIN, Some(true), Some(0), &CFG), vec![Event::Snapshot]);
        let ev = w.tick(t0 + 121 * MIN, Some(false), Some(0), &CFG);
        assert!(matches!(ev.as_slice(), [Event::WelcomeBack { away_ms }] if *away_ms >= 120 * MIN));
    }

    #[test]
    fn unknown_lock_state_still_works_with_idle() {
        let mut w = Watch::default();
        let t0 = 1_000_000_000;
        w.tick(t0, None, Some(0), &CFG);
        w.tick(t0 + 60_000, None, Some(60), &CFG);
        assert_eq!(w.tick(t0 + 90_000, None, Some(330), &CFG), vec![Event::Snapshot]);
    }

    #[test]
    fn parsers() {
        let plist = "<dict><key>CGSSessionScreenIsLocked</key>\n\t\t<true/><key>x</key></dict>";
        assert!(parse_locked(plist));
        assert!(!parse_locked("<key>CGSSessionScreenIsLocked</key><false/>"));
        assert!(!parse_locked("<dict></dict>"));
        let txt = "  |   \"HIDIdleTime\" = 125000000000\n  |   \"Other\" = 1";
        assert_eq!(parse_idle(txt), Some(125));
        assert_eq!(parse_idle("nothing"), None);
    }
}
