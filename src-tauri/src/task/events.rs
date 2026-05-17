use crate::platform::{DownloadEvent, EventSink};
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct MemoryEventSink {
    events: Arc<Mutex<Vec<DownloadEvent>>>,
}

impl MemoryEventSink {
    pub fn events(&self) -> Vec<DownloadEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl EventSink for MemoryEventSink {
    fn emit(&self, event: DownloadEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_event_sink_collects_cloned_events() {
        let sink = MemoryEventSink::default();
        let cloned = sink.clone();

        sink.emit(DownloadEvent::Log("queued".into()));
        cloned.emit(DownloadEvent::State("downloading".into()));

        assert_eq!(
            sink.events(),
            vec![
                DownloadEvent::Log("queued".into()),
                DownloadEvent::State("downloading".into()),
            ]
        );
    }
}
