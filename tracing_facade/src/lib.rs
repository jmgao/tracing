//! A facade for tracing.
//!
//! This crate provides a pluggable API for tracing, akin to what [log] does for logging.
//!
//! ### Example
//! ```
//! #[macro_use]
//! extern crate tracing_facade;
//!
//! #[macro_use]
//! extern crate serde_json;
//!
//! use std::borrow::Cow;
//! use std::sync::{Arc, Mutex};
//!
//! #[derive(Clone, Debug)]
//! struct Event {
//!   name: String,
//!   kind: tracing_facade::EventKind,
//!   metadata: tracing_facade::Metadata,
//! }
//!
//! impl<'a> From<tracing_facade::Event<'a>> for Event {
//!   fn from(event: tracing_facade::Event) -> Self {
//!     Event {
//!       name: event.name.into_owned(),
//!       kind: event.kind,
//!       metadata: event.metadata,
//!     }
//!   }
//! }
//!
//! struct Tracer {
//!   events: Arc<Mutex<Vec<Event>>>
//! }
//!
//! impl Tracer {
//!   fn new() -> (Tracer, Arc<Mutex<Vec<Event>>>) {
//!     let vec = Arc::new(Mutex::new(Vec::new()));
//!     let tracer = Tracer {
//!       events: Arc::clone(&vec)
//!     };
//!     (tracer, vec)
//!   }
//! }
//!
//!
//! impl tracing_facade::Tracer for Tracer {
//!   fn supports_metadata(&self) -> bool {
//!     true
//!   }
//!
//!   fn record_event(&self, event: tracing_facade::Event) {
//!     self.events.lock().unwrap().push(event.into());
//!   }
//!
//!   fn flush(&self) {}
//! }
//!
//! fn main() {
//!   let (tracer, tracer_events) = Tracer::new();
//!   tracing_facade::set_boxed_tracer(Box::new(tracer));
//!   {
//!     trace_scoped!("foo");
//!     trace_begin!("bar", "value": 42);
//!     trace_end!("bar");
//!   }
//!
//!   let events = tracer_events.lock().unwrap().clone();
//!   assert_eq!(events.len(), 4);
//!   assert_eq!(events[0].name, "foo");
//!   assert_eq!(events[0].kind, tracing_facade::EventKind::SyncBegin);
//!   assert_eq!(events[0].metadata.as_json(), None);
//!
//!   assert_eq!(events[1].name, "bar");
//!   assert_eq!(events[1].kind, tracing_facade::EventKind::SyncBegin);
//!   assert_eq!(events[1].metadata.as_json(), Some(&json!({"value": 42})));
//!
//!   assert_eq!(events[2].name, "bar");
//!   assert_eq!(events[2].kind, tracing_facade::EventKind::SyncEnd);
//!   assert_eq!(events[2].metadata.as_json(), None);
//!
//!   assert_eq!(events[3].name, "foo");
//!   assert_eq!(events[3].kind, tracing_facade::EventKind::SyncEnd);
//!   assert_eq!(events[3].metadata.as_json(), None);
//! }
//! ```

use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

mod macros;
pub use macros::*;

pub enum Error {}

pub trait Tracer: Sync + Send {
  fn is_enabled(&self) -> bool {
    true
  }

  fn supports_metadata(&self) -> bool {
    false
  }

  fn record_event(&self, event: Event);
  fn flush(&self);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
  SyncBegin,
  SyncEnd,
}

pub struct Event<'a> {
  pub name: Cow<'a, str>,
  pub kind: EventKind,
  pub metadata: Metadata,
}

#[derive(Clone, Debug)]
pub struct Metadata {
  json: Option<serde_json::Value>,
}

impl Metadata {
  pub fn as_json(&self) -> Option<&serde_json::Value> {
    self.json.as_ref()
  }

  pub fn into_json(self) -> Option<serde_json::Value> {
    self.json
  }

  pub fn from_json(json: serde_json::Value) -> Metadata {
    Metadata { json: Some(json) }
  }
}

impl Default for Metadata {
  fn default() -> Self {
    Metadata { json: None }
  }
}

pub fn is_enabled() -> bool {
  loop {
    match STATE.load(Ordering::Acquire) {
      UNINITIALIZED => return false,
      INITIALIZING => std::thread::yield_now(),
      INITIALIZED => return get_tracer_assume_initialized().is_enabled(),
      other => panic!("unexpected tracing_facade::STATE value: {}", other),
    }
  }
}

pub fn supports_metadata() -> bool {
  if is_enabled() {
    get_tracer_assume_initialized().supports_metadata()
  } else {
    false
  }
}

pub fn record_event(event: Event) {
  if is_enabled() {
    get_tracer_assume_initialized().record_event(event)
  }
}

pub fn flush() {
  if is_enabled() {
    get_tracer_assume_initialized().flush()
  }
}

static mut TRACER: Option<&'static Tracer> = None;

static STATE: AtomicUsize = AtomicUsize::new(UNINITIALIZED);

/// The tracer hasn't been initialized yet.
const UNINITIALIZED: usize = 0;

/// The tracer has been initialized.
const INITIALIZED: usize = 1;

/// The tracer is in the process of initializing.
const INITIALIZING: usize = 2;

fn get_tracer_assume_initialized() -> &'static Tracer {
  unsafe { TRACER.unwrap() }
}

pub fn set_tracer(tracer: &'static Tracer) {
  set_tracer_impl(tracer);
}

pub fn set_boxed_tracer(tracer: Box<Tracer>) {
  let raw = Box::into_raw(tracer);
  set_tracer_impl(unsafe { &*raw });
}

fn set_tracer_impl(tracer: &'static Tracer) {
  loop {
    match STATE.compare_exchange(UNINITIALIZED, INITIALIZING, Ordering::AcqRel, Ordering::Relaxed) {
      Ok(_) => {
        // We've recorded ourselves as the initializer.
        unsafe {
          TRACER = Some(tracer);
        }
        STATE
          .compare_exchange(INITIALIZING, INITIALIZED, Ordering::AcqRel, Ordering::Relaxed)
          .unwrap();
        return;
      }

      Err(UNINITIALIZED) => {
        // This should be impossible.
        unreachable!();
      }

      Err(INITIALIZED) | Err(INITIALIZING) => {
        panic!("attempted to set a tracer after the tracing system was already initialized");
      }

      Err(_) => {
        // This should also be impossible.
        unreachable!();
      }
    }
  }
}
