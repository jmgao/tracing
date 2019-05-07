#[doc(hidden)]
pub use scopeguard::guard;

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
