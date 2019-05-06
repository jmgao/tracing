#[doc(hidden)]
pub use scopeguard::guard;

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
      trace_begin!(name.clone());
      Some($crate::guard(name, move |name| {
        trace_end!(name);
      }))
    } else {
      None
    };
  };
}
