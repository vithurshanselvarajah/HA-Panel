use log::{Level, LevelFilter, Log, Metadata, Record};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

pub const CAPACITY: usize = 2048;

#[derive(Debug, Clone)]
pub struct Entry {
    pub level: Level,

    pub target: String,

    pub message: String,

    pub timestamp_ms: u128,
}

#[derive(Debug)]
pub struct Ring {
    entries: Mutex<Vec<Entry>>,
    capacity: usize,
}

impl Ring {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&self, entry: Entry) {
        let mut entries = match self.entries.lock() {
            Ok(guard) => guard,

            Err(poisoned) => poisoned.into_inner(),
        };

        if entries.len() >= self.capacity {
            entries.remove(0);
        }
        entries.push(entry);
    }

    pub fn entries(&self) -> Vec<Entry> {
        match self.entries.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn entries_at_or_above(&self, level: Level) -> Vec<Entry> {
        self.entries()
            .into_iter()
            .filter(|entry| entry.level <= level)
            .collect()
    }

    pub fn clear(&self) {
        match self.entries.lock() {
            Ok(mut guard) => guard.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    pub fn len(&self) -> usize {
        match self.entries.lock() {
            Ok(guard) => guard.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

static RING: OnceLock<Ring> = OnceLock::new();

pub fn ring() -> &'static Ring {
    RING.get_or_init(|| Ring::new(CAPACITY))
}

struct Sink {
    stderr_level: Level,
}

impl Log for Sink {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        ring().push(Entry {
            level: record.level(),
            target: record.target().to_string(),
            message: format!("{}", record.args()),
            timestamp_ms: now_ms(),
        });

        if record.level() <= self.stderr_level {
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(
                stderr,
                "[{:<5}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn install(stderr_level: Level) {
    let sink = Box::leak(Box::new(Sink { stderr_level }));

    if log::set_logger(sink).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

pub fn level_from_name(name: &str) -> Option<Level> {
    match name.trim().to_lowercase().as_str() {
        "error" => Some(Level::Error),
        "warn" => Some(Level::Warn),
        "info" => Some(Level::Info),
        "debug" => Some(Level::Debug),
        "trace" => Some(Level::Trace),
        _ => None,
    }
}

pub fn level_name() -> &'static str {
    match log::max_level() {
        LevelFilter::Off => "off",
        LevelFilter::Error => "error",
        LevelFilter::Warn => "warn",
        LevelFilter::Info => "info",
        LevelFilter::Debug => "debug",
        LevelFilter::Trace => "trace",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_drops_the_oldest_record_when_it_is_full() {
        let ring = Ring::new(3);
        for index in 0..5 {
            ring.push(Entry {
                level: Level::Info,
                target: "test".to_string(),
                message: index.to_string(),
                timestamp_ms: index,
            });
        }

        assert_eq!(ring.len(), 3);
        let messages: Vec<String> = ring.entries().into_iter().map(|e| e.message).collect();
        assert_eq!(messages, vec!["2", "3", "4"]);
    }

    #[test]
    fn filtering_keeps_the_more_severe_records() {
        let ring = Ring::new(8);
        for (level, message) in [
            (Level::Error, "e"),
            (Level::Warn, "w"),
            (Level::Info, "i"),
            (Level::Debug, "d"),
        ] {
            ring.push(Entry {
                level,
                target: "test".to_string(),
                message: message.to_string(),
                timestamp_ms: 0,
            });
        }

        let at_warn: Vec<String> = ring
            .entries_at_or_above(Level::Warn)
            .into_iter()
            .map(|e| e.message)
            .collect();
        assert_eq!(at_warn, vec!["e", "w"]);
    }

    #[test]
    fn clearing_empties_the_ring() {
        let ring = Ring::new(4);
        ring.push(Entry {
            level: Level::Info,
            target: "test".to_string(),
            message: "x".to_string(),
            timestamp_ms: 0,
        });
        assert!(!ring.is_empty());
        ring.clear();
        assert!(ring.is_empty());
    }

    #[test]
    fn a_ring_of_zero_capacity_still_holds_one_record() {
        let ring = Ring::new(0);
        ring.push(Entry {
            level: Level::Info,
            target: "test".to_string(),
            message: "x".to_string(),
            timestamp_ms: 0,
        });
        assert_eq!(ring.len(), 1);
    }
}
