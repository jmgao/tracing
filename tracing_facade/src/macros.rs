#[macro_export]
macro_rules! trace_begin {
  ($name: expr) => {
    if $crate::is_enabled() {
      let event = $crate::Event {
        name: $name.into(),
        kind: $crate::EventKind::SyncBegin,
        metadata: $crate::Metadata {},
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
        metadata: $crate::Metadata {},
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
      let event = $crate::Event {
        name: name.clone(),
        kind: $crate::EventKind::SyncBegin,
        metadata: $crate::Metadata {},
      };
      $crate::record_event(event);

      Some($crate::guard(name, |name| {
        let event = $crate::Event {
          name,
          kind: $crate::EventKind::SyncEnd,
          metadata: $crate::Metadata {},
        };
        $crate::record_event(event);
      }))
    } else {
      None
    };
  };
}

#[doc(hidden)]
pub use scopeguard::guard;
