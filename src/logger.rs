use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use log::{Level, LevelFilter, Metadata, Record, Log};
use std::sync::LazyLock;

/// A dual logger that writes to both the console and a log file.
struct DualLogger {
    file: Mutex<Option<BufWriter<File>>>,
}

impl DualLogger {
    fn new() -> Self {
        let file = if let Ok(path) = std::env::var("WRAPPER_LOG_FILE") {
            match File::options().create(true).append(true).open(&path) {
                Ok(f) => {
                    eprintln!("[WRAPPER] Logging to file: {}", path);
                    Some(BufWriter::new(f))
                }
                Err(e) => {
                    eprintln!("[WRAPPER] Failed to open log file '{}': {}", path, e);
                    None
                }
            }
        } else {
            None
        };
        DualLogger {
            file: Mutex::new(file),
        }
    }
}

impl Log for DualLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        let level = record.level();
        let target = record.target();
        let args = record.args();

        let message = format!("[{}] [{}] {}", level, target, args);

        // Write to stdout (like env_logger default)
        println!("{}", message);

        // Write to file if configured
        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut writer) = *guard {
                let _ = writeln!(writer, "{}", message);
                let _ = writer.flush();
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut writer) = *guard {
                let _ = writer.flush();
            }
        }
    }
}

static LOGGER: LazyLock<DualLogger> = LazyLock::new(DualLogger::new);

/// Initialize the dual logger. Call this at the start of each binary.
///
/// The verbosity is taken from the standard `RUST_LOG` variable (e.g. `trace`,
/// `debug`, `info`, `warn`). When `RUST_LOG` is unset or holds an unrecognised
/// value the default level is `error`, so the wrapper stays quiet unless told
/// otherwise.
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&*LOGGER).map(|()| {
        let level = match std::env::var("RUST_LOG").as_deref() {
            Ok("trace") => LevelFilter::Trace,
            Ok("debug") => LevelFilter::Debug,
            Ok("info") => LevelFilter::Info,
            Ok("warn") => LevelFilter::Warn,
            Ok("error") => LevelFilter::Error,
            _ => LevelFilter::Error,
        };
        log::set_max_level(level);
    })
}
