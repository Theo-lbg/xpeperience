//! Limitation des tentatives de connexion (protection contre le devinage de mots de passe).
//!
//! Chaque clé (adresse IP, compte) cumule les échecs sur une fenêtre glissante ; au-delà du seuil,
//! elle est bloquée un moment. Un succès efface le compteur du compte. L'état est en mémoire : un
//! redémarrage du serveur remet les compteurs à zéro, ce qui est acceptable pour cet usage.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Entry {
    failures: u32,
    window_start: Instant,
    locked_until: Option<Instant>,
}

pub struct Limiter {
    max_failures: u32,
    window: Duration,
    lock: Duration,
    entries: Mutex<HashMap<String, Entry>>,
}

const MAX_ENTRIES: usize = 20_000;

impl Limiter {
    pub fn new(max_failures: u32, window: Duration, lock: Duration) -> Self {
        Self { max_failures, window, lock, entries: Mutex::new(HashMap::new()) }
    }

    /// Réglage par défaut : 6 échecs en 10 minutes bloquent la clé pendant 15 minutes.
    pub fn standard() -> Self {
        Self::new(6, Duration::from_secs(600), Duration::from_secs(900))
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        self.entries.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// `Err(délai)` si la clé est bloquée : durée restante avant une nouvelle tentative.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        let now = Instant::now();
        match self.entries().get(key).and_then(|e| e.locked_until) {
            Some(until) if until > now => Err(until - now),
            _ => Ok(()),
        }
    }

    pub fn failure(&self, key: &str) {
        let now = Instant::now();
        let mut map = self.entries();
        if map.len() >= MAX_ENTRIES {
            map.retain(|_, e| e.locked_until.is_some_and(|u| u > now) || now.duration_since(e.window_start) < self.window);
            if map.len() >= MAX_ENTRIES {
                return; // saturé : mieux vaut ne plus compter que consommer la mémoire sans limite
            }
        }
        let e = map.entry(key.to_string()).or_insert(Entry { failures: 0, window_start: now, locked_until: None });
        if now.duration_since(e.window_start) > self.window {
            e.failures = 0;
            e.window_start = now;
            e.locked_until = None;
        }
        e.failures += 1;
        if e.failures >= self.max_failures {
            e.locked_until = Some(now + self.lock);
        }
    }

    pub fn success(&self, key: &str) {
        self.entries().remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_after_threshold_and_success_resets() {
        let l = Limiter::new(3, Duration::from_secs(60), Duration::from_secs(60));
        for _ in 0..2 {
            l.failure("a");
        }
        assert!(l.check("a").is_ok());
        l.failure("a");
        assert!(l.check("a").is_err());
        assert!(l.check("b").is_ok(), "les autres clés ne sont pas touchées");
        l.success("a");
        assert!(l.check("a").is_ok());
    }

    #[test]
    fn lock_expires() {
        let l = Limiter::new(1, Duration::from_secs(60), Duration::from_millis(30));
        l.failure("a");
        assert!(l.check("a").is_err());
        std::thread::sleep(Duration::from_millis(50));
        assert!(l.check("a").is_ok());
    }
}
