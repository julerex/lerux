use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{prelude::*, BufWriter};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::{fmt, io};

use log::{Metadata, Record};
use smallvec::SmallVec;
use termion::color;

static LOCAL_OFFSET: OnceLock<chrono::FixedOffset> = OnceLock::new();

/// An output that will be logged to. The two major outputs for most Redox system programs are
/// usually the log file, and the global stdout.
pub struct Output {
    // the actual endpoint to write to.
    endpoint: Mutex<Box<dyn Write + Send + 'static>>,

    // useful for devices like BufWrite or BufRead. You don't want the log file to never but
    // written until the program exists.
    flush_on_newline: bool,

    // specifies the maximum log level possible
    filter: log::LevelFilter,

    // specifies whether the file should contain ASCII escape codes
    ansi: bool,
}
impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Output")
            .field("endpoint", &"opaque")
            .field("flush_on_newline", &self.flush_on_newline)
            .field("filter", &self.filter)
            .field("ansi", &self.ansi)
            .finish()
    }
}

pub struct OutputBuilder {
    endpoint: Box<dyn Write + Send + 'static>,
    flush_on_newline: Option<bool>,
    filter: Option<log::LevelFilter>,
    ansi: Option<bool>,
}
impl OutputBuilder {
    pub fn in_redox_logging_scheme<A, B, C>(
        category: A,
        subcategory: B,
        logfile: C,
    ) -> Result<Self, io::Error>
    where
        A: AsRef<OsStr>,
        B: AsRef<OsStr>,
        C: AsRef<OsStr>,
    {
        if !cfg!(target_os = "redox") {
            return Ok(Self::with_endpoint(Vec::new()));
        }

        let mut path = PathBuf::from("/scheme/logging/");
        path.push(category.as_ref());
        path.push(subcategory.as_ref());
        path.push(logfile.as_ref());
        path.set_extension("log");

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(Self::with_endpoint(BufWriter::new(File::create(path)?)))
    }

    pub fn stdout() -> Self {
        Self::with_endpoint(io::stdout())
    }
    pub fn stderr() -> Self {
        Self::with_endpoint(io::stderr())
    }

    pub fn with_endpoint<T>(endpoint: T) -> Self
    where
        T: Write + Send + 'static,
    {
        Self::with_dyn_endpoint(Box::new(endpoint))
    }
    pub fn with_dyn_endpoint(endpoint: Box<dyn Write + Send + 'static>) -> Self {
        Self {
            endpoint,
            flush_on_newline: None,
            filter: None,
            ansi: None,
        }
    }
    pub fn flush_on_newline(mut self, flush: bool) -> Self {
        self.flush_on_newline = Some(flush);
        self
    }
    pub fn with_filter(mut self, filter: log::LevelFilter) -> Self {
        self.filter = Some(filter);
        self
    }
    pub fn with_ansi_escape_codes(mut self) -> Self {
        self.ansi = Some(true);
        self
    }
    pub fn build(self) -> Output {
        Output {
            endpoint: Mutex::new(self.endpoint),
            filter: self.filter.unwrap_or(log::LevelFilter::Info),
            flush_on_newline: self.flush_on_newline.unwrap_or(true),
            ansi: self.ansi.unwrap_or(false),
        }
    }
}

const AVG_OUTPUTS: usize = 2;

#[derive(Debug, Default)]
pub struct RedoxLogger {
    outputs: SmallVec<[Output; AVG_OUTPUTS]>,
    min_filter: Option<log::LevelFilter>,
    max_filter: Option<log::LevelFilter>,
    max_level_in_use: Option<log::LevelFilter>,
    min_level_in_use: Option<log::LevelFilter>,
    process_name: Option<String>,
}

impl RedoxLogger {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn init_timezone() {
        LOCAL_OFFSET.get_or_init(|| *chrono::Local::now().offset());
    }
    fn adjust_output_level(
        max_filter: Option<log::LevelFilter>,
        min_filter: Option<log::LevelFilter>,
        max_in_use: &mut Option<log::LevelFilter>,
        min_in_use: &mut Option<log::LevelFilter>,
        output: &mut Output,
    ) {
        if let Some(max) = max_filter {
            output.filter = std::cmp::max(output.filter, max);
        }
        if let Some(min) = min_filter {
            output.filter = std::cmp::min(output.filter, min);
        }
        match max_in_use {
            &mut Some(ref mut max) => *max = std::cmp::max(output.filter, *max),
            max @ &mut None => *max = Some(output.filter),
        }
        match min_in_use {
            &mut Some(ref mut min) => *min = std::cmp::min(output.filter, *min),
            min @ &mut None => *min = Some(output.filter),
        }
    }
    pub fn with_output(mut self, mut output: Output) -> Self {
        Self::adjust_output_level(
            self.max_filter,
            self.min_filter,
            &mut self.max_level_in_use,
            &mut self.min_level_in_use,
            &mut output,
        );
        self.outputs.push(output);
        self
    }
    pub fn with_min_level_override(mut self, min: log::LevelFilter) -> Self {
        self.min_filter = Some(min);
        for output in &mut self.outputs {
            Self::adjust_output_level(
                self.max_filter,
                self.min_filter,
                &mut self.max_level_in_use,
                &mut self.min_level_in_use,
                output,
            );
        }
        self
    }
    pub fn with_max_level_override(mut self, max: log::LevelFilter) -> Self {
        self.max_filter = Some(max);
        for output in &mut self.outputs {
            Self::adjust_output_level(
                self.max_filter,
                self.min_filter,
                &mut self.max_level_in_use,
                &mut self.min_level_in_use,
                output,
            );
        }
        self
    }
    pub fn with_process_name(mut self, name: String) -> Self {
        self.process_name = Some(name);
        self
    }
    pub fn enable(self) -> Result<&'static Self, log::SetLoggerError> {
        let leak = Box::leak(Box::new(self));
        log::set_logger(leak)?;
        if let Some(max) = leak.max_level_in_use {
            log::set_max_level(max);
        } else {
            log::set_max_level(log::LevelFilter::Off);
        }
        Ok(leak)
    }
    fn write_record<W: Write>(
        ansi: bool,
        record: &Record,
        process_name: Option<&str>,
        writer: &mut W,
    ) -> io::Result<()> {
        use log::Level;
        use termion::style;

        // TODO: Log offloading to another thread or thread pool, maybe?
        // Time & Time Zone Formatting
        let offset = LOCAL_OFFSET.get_or_init(|| *chrono::Local::now().offset());
        let now_local = chrono::Utc::now().with_timezone(offset);
        let time = now_local.format("%Y-%m-%dT%H-%M-%S%.3f");
        let mut zone = format!("{}", now_local.format("%:z"));
        if zone.as_str() == "+00:00" {
            zone = "Z".to_string();
        }

        let target = record.module_path().unwrap_or(record.target());
        let level = record.level();
        let message = record.args();

        let reset = color::Fg(color::Reset);

        let show_lines = true;
        let line_number = if show_lines { record.line() } else { None };

        let process_name = process_name.unwrap_or("");
        let line = &LineFmt(line_number, false);

        if ansi {
            let time_color = color::Fg(color::LightWhite);
            let zone_color = color::Fg(color::White);

            let trace_col = color::Fg(color::LightBlack);
            let debug_col = color::Fg(color::White);
            let info_col = color::Fg(color::LightBlue);
            let warn_col = color::Fg(color::LightYellow);
            let err_col = color::Fg(color::LightRed);

            let level_color: &dyn fmt::Display = match level {
                Level::Trace => &trace_col,
                Level::Debug => &debug_col,
                Level::Info => &info_col,
                Level::Warn => &warn_col,
                Level::Error => &err_col,
            };

            let dim_white = color::Fg(color::White);
            let bright_white = color::Fg(color::LightWhite);
            let regular_style = "";
            let bold_style = style::Bold;

            let [message_color, message_style]: [&dyn fmt::Display; 2] = match level {
                Level::Trace | Level::Debug => [&dim_white, &regular_style],
                Level::Info | Level::Warn | Level::Error => [&bright_white, &bold_style],
            };
            let target_color = color::Fg(color::White);

            let i = style::Italic;
            let b = style::Bold;
            let r = reset;
            let rs = style::Reset;

            writeln!(
                writer,
                "{time}{zone} [{target}{line} {level}] {msg}",
                time = format_args!("{i}{time_color}{time}{rs}{r}"),
                zone = format_args!("{i}{zone_color}{zone}{rs}{r}"),
                level = format_args!("{b}{level_color}{level}{rs}{r}"),
                target = format_args!("{target_color}{process_name}@{target}{r}"),
                msg = format_args!("{message_style}{message_color}{message}{rs}{r}"),
            )
        } else {
            writeln!(
                writer,
                "{time}{zone} [{process_name}@{target}{line} {level}] {message}",
            )
        }
    }
}

impl log::Log for RedoxLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.max_level_in_use
            .map(|min| metadata.level() >= min)
            .unwrap_or(false)
            && self
                .min_level_in_use
                .map(|max| metadata.level() <= max)
                .unwrap_or(false)
    }
    fn log(&self, record: &Record) {
        for output in &self.outputs {
            if record.metadata().level() <= output.filter {
                let mut endpoint_guard = match output.endpoint.lock() {
                    Ok(e) => e,
                    // poison error
                    _ => continue,
                };

                let _ = Self::write_record(
                    output.ansi,
                    record,
                    self.process_name.as_deref(),
                    &mut *endpoint_guard,
                );

                if output.flush_on_newline {
                    let _ = endpoint_guard.flush();
                }
            }
        }
    }
    fn flush(&self) {
        for output in &self.outputs {
            match output.endpoint.lock() {
                Ok(ref mut e) => {
                    let _ = e.flush();
                }
                _ => continue,
            }
        }
    }
}

struct LineFmt(Option<u32>, bool);
impl fmt::Display for LineFmt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(line) = self.0 {
            if self.1 {
                // ansi escape codes
                let color = color::Fg(color::LightBlack);
                let reset = color::Fg(color::Reset);
                write!(f, "{color}:{line}{reset}")
            } else {
                // no ansi escape codes
                write!(f, ":{line}")
            }
        } else {
            write!(f, "")
        }
    }
}
