#[doc(hidden)]
pub use scopeguard::guard;

/// Records the end of a synchronous duration.
///
/// Accepts an expression of a type that implements [Into<Cow<str>>], with optional metadata
/// following. Uses of `trace_begin` and `trace_end` must be balanced; in most cases, [trace_scoped]
/// should be used instead.
///
/// The behavior of Metadata specification depends on the implementation of [Tracer] being used.
/// Chromium's trace event format will merge metadata from beginning and end, preferring values from
/// the end in the case of conflict.
///
/// # Example
/// ```
/// # #[macro_use] extern crate tracing_facade;
/// trace_begin!("foo");
/// trace_begin!("bar", "value": 42);
/// trace_end!("bar", "value": 123, "values": [1, 2, 3]);
/// trace_end!("foo");
/// ```
#[macro_export]
macro_rules! trace_begin {
  ($name: expr) => {
    if $crate::is_enabled() {
      let event = $crate::Event {
        name: $name.into(),
        kind: $crate::EventKind::SyncBegin,
        metadata: $crate::Metadata::default(),
      };
      $crate::record_event(event);
    }
  };

  ($name: expr, $($metadata: tt)+) => {
    if $crate::is_enabled() {
      let metadata = if $crate::supports_metadata() {
        $crate::Metadata::from_json(serde_json::json!({$($metadata)+}))
      } else {
        $crate::Metadata::default()
      };

      let event = $crate::Event {
        name: $name.into(),
        kind: $crate::EventKind::SyncBegin,
        metadata,
      };
      $crate::record_event(event);
    }
  };
}

/// Records the end of a synchronous duration.
///
/// Accepts an expression of a type that implements [Into<Cow<str>>], with optional metadata
/// following. Uses of `trace_begin` and `trace_end` must be balanced; in most cases, [trace_scoped]
/// should be used instead.
///
/// The behavior of Metadata specification depends on the implementation of [Tracer] being used.
/// Chromium's trace event format will merge metadata from beginning and end, preferring values from
/// the end in the case of conflict.
///
/// # Example
/// ```
/// # #[macro_use] extern crate tracing_facade;
/// trace_begin!("foo");
/// trace_begin!("bar", "value": 42);
/// trace_end!("bar", "value": 123, "values": [1, 2, 3]);
/// trace_end!("foo");
/// ```
#[macro_export]
macro_rules! trace_end {
  ($name: expr) => {
    if $crate::is_enabled() {
      let event = $crate::Event {
        name: $name.into(),
        kind: $crate::EventKind::SyncEnd,
        metadata: $crate::Metadata::default(),
      };
      $crate::record_event(event);
    }
  };

  ($name: expr, $($metadata: tt)+) => {
    if $crate::is_enabled() {
      let metadata = if $crate::supports_metadata() {
        $crate::Metadata::from_json(serde_json::json!({$($metadata)+}))
      } else {
        $crate::Metadata::default()
      };

      let event = $crate::Event {
        name: $name.into(),
        kind: $crate::EventKind::SyncEnd,
        metadata,
      };
      $crate::record_event(event);
    }
  };
}

/// Traces in a given scope.
///
/// [trace_scoped] calls [trace_begin], and then constructs a scope guard that calls [trace_end]
/// upon the exit of the scope. Metadata, if specified, is provided to only [trace_begin].
#[macro_export]
macro_rules! trace_scoped {
  ($name: expr) => {
    let guard = if $crate::is_enabled() {
      let name: std::borrow::Cow<str> = $name.into();
      $crate::trace_begin!(name.clone());
      Some($crate::guard(name, move |name| {
        $crate::trace_end!(name);
      }))
    } else {
      None
    };
  };

  ($name: expr, $($metadata: tt)+) => {
    let guard = if $crate::is_enabled() {
      let name: std::borrow::Cow<str> = $name.into();
      let metadata = if $crate::supports_metadata() {
        $crate::Metadata::from_json(serde_json::json!({$($metadata)+}))
      } else {
        $crate::Metadata::default()
      };
      $crate::trace_begin!(name.clone(), metadata);
      Some($crate::guard(name, move |name| {
        $crate::trace_end!(name);
      }))
    } else {
      None
    };
  };
}
