//! A facade for tracing.
//!
//! This crate provides a pluggable API for tracing, akin to what [log] does for logging.
//!
//! Some available implementations are:
//!  * [tracing_chromium] - to emit Chromium's trace event format
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

/// A trait encompassing the operations required for tracing.
pub trait Tracer: Sync + Send {
  /// Determines whether the [Tracer] is enabled.
  ///
  /// The trace macros use this to avoid doing work when unneeded.
  fn is_enabled(&self) -> bool {
    true
  }

  /// Specifies whether the [Tracer] can handle additional metadata.
  ///
  /// The trace macros use this to avoid constructing a [Metadata] object which will get immediately
  /// thrown away.
  fn supports_metadata(&self) -> bool {
    false
  }

  /// Record an [Event] to the Tracer.
  fn record_event(&self, event: Event);

  /// Flush any previously recorded [Event]s to the Tracer.
  fn flush(&self);
}

/// An enum representing the types of [Event]s that can occur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
  /// The beginning of a synchronous duration.
  ///
  /// This represents the beginning of a duration on a particular thread. Durations can be nested,
  /// but must not overlap.
  SyncBegin,

  /// The end of a synchronous duration.
  ///
  /// This represents the end of a duration on a particular thread. Durations can be nested,
  /// but must not overlap.
  SyncEnd,
}

/// An event to trace.
pub struct Event<'a> {
  /// The name of the [Event].
  pub name: Cow<'a, str>,

  /// The type of [Event] which occurred.
  pub kind: EventKind,

  /// [Metadata] attached to the event.
  pub metadata: Metadata,
}

/// A struct containing metadata for an event.
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

/// Determines whether a [Tracer] has been installed, and if it is currently enabled.
///
/// If a [Tracer] has been installed, returns the result of [Tracer::is_enabled].
/// Otherwise, returns false.
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

/// Determines whether a [Tracer] has been installed, and if it supports metadata.
///
/// If a [Tracer] has been installed, returns the result of [Tracer::supports_metadata].
/// Otherwise, returns false.
pub fn supports_metadata() -> bool {
  if is_enabled() {
    get_tracer_assume_initialized().supports_metadata()
  } else {
    false
  }
}

/// Records an Event to the installed [Tracer].
///
/// If a [Tracer] has been installed, invokes [Tracer::record_event] on it.
/// Otherwise, does nothing.
pub fn record_event(event: Event) {
  if is_enabled() {
    get_tracer_assume_initialized().record_event(event)
  }
}

/// Flushes the installed [Tracer].
///
/// If a [Tracer] has been installed, invokes [Tracer::flush] on it.
/// Otherwise, does nothing.
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

/// Installs a [Tracer].
///
/// Installation can only occur once; subsequent installations will panic.
pub fn set_tracer(tracer: &'static Tracer) {
  set_tracer_impl(tracer);
}

/// Installs a [Tracer] contained in a [Box].
///
/// The contained [Tracer] will never be destroyed.
/// Installation can only occur once; subsequent installations will panic.
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
