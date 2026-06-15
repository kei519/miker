//! Provides logging for kernel.

use alloc::{collections::vec_deque::VecDeque, format, string::String};
use log::{Level, LevelFilter, Log};
use util::{asmfunc, error, error::Result, sync::InterruptFreeMutex};

use crate::timer;

/// [Logger] for kernel.
static LOGGER: Logger = Logger {
    entries: InterruptFreeMutex::new(VecDeque::new()),
};

/// Initialize [logger][crate::logger] module.
pub fn init() -> Result<()> {
    log::set_max_level(LevelFilter::Trace);
    if let Err(e) = log::set_logger(&LOGGER) {
        error!(e);
    }
    Ok(())
}

/// Provides logging implementation for kernel.
struct Logger {
    /// Collect all [LogEntry] coming from kernel.
    // TODO: We want to depend nothing to store LogEntry because depended modules cannot use
    //       logging.
    entries: InterruptFreeMutex<VecDeque<LogEntry>>,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        use util::bitfield::BitField as _;

        if !self.enabled(record.metadata()) {
            return;
        }

        let entry = LogEntry {
            level: record.level(),
            time_stamp: timer::get_timestamp(),
            module: "kernel",
            content: format!("{}", record.args()),
        };

        // We use UART for logging in early stage, but use other means step by step.
        //
        // 1. Output logs directly with UART.
        // 2. Just provide the character device (?) file that outputs all logs (thorough fs
        //   service), and log service prints them througu UART.
        // 3. Log service prints logs through fb service.
        {
            const UART_MAX_LEVEL: Level = Level::Info;
            const UART_BASE_PORT: u16 = 0x3f8;
            const LINE_PORT: u16 = UART_BASE_PORT + 5;

            let micros = entry.time_stamp % 1_000_000_000 / 1_000;
            let secs = entry.time_stamp / 1_000_000_000;

            if entry.level <= UART_MAX_LEVEL {
                for b in format!(
                    "[{:6}.{:06}, {:>5}] ({}) {}\n",
                    secs, micros, entry.level, entry.module, entry.content,
                )
                .as_bytes()
                {
                    while !asmfunc::io_inb(LINE_PORT).get_bit(5) {
                        core::hint::spin_loop();
                    }
                    asmfunc::io_outb(UART_BASE_PORT, *b);
                }
            }
        }

        let mut entries = self.entries.lock();
        entries.push_back(entry);
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        self.flush();
    }
}

#[derive(Debug)]
struct LogEntry {
    level: Level,
    time_stamp: u64,
    module: &'static str,
    content: String,
}
