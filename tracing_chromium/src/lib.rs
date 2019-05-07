//! An implementation for [tracing_facade] that emits the [Chromium Trace Event Format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview).

use std::ops::DerefMut;
use std::sync::Mutex;

use serde::Serialize;

use tracing_facade::{Event, EventKind};

pub struct Tracer {
  output: Mutex<Box<std::io::Write + Send>>,
}

impl tracing_facade::Tracer for Tracer {
  fn supports_metadata(&self) -> bool {
    true
  }

  fn record_event(&self, event: Event) {
    let mut lock = self.output.lock().unwrap();
    write_event(lock.deref_mut(), event);
  }

  fn flush(&self) {
    let _ = self.output.lock().unwrap().flush();
  }
}

impl Tracer {
  pub fn from_output(mut output: Box<std::io::Write + Send>) -> Tracer {
    let _ = output.write_all(b"[");
    Tracer {
      output: Mutex::new(output),
    }
  }
}

#[derive(Serialize)]
struct Record<'a> {
  name: &'a str,
  ph: &'static str,
  pid: u32,
  tid: u32,
  ts: u64,
  arg: Option<serde_json::Value>,
}

fn write_event(mut output: &mut std::io::Write, event: Event) {
  let now = time::precise_time_ns() / 1000;
  let phase = match event.kind {
    EventKind::SyncBegin => "B",
    EventKind::SyncEnd => "E",
  };

  let record = Record {
    name: &event.name,
    ph: phase,
    pid: std::process::id(),
    tid: gettid::gettid() as u32,
    ts: now,
    arg: event.metadata.into_json(),
  };

  // Ignore errors.
  // There isn't a good way for them to be handled, but we don't want to blow up.
  let _ = serde_json::to_writer(&mut output, &record);
  let _ = output.write_all(b",");
}
