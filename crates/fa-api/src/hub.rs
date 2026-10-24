use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use fa_domain::HouseholdId;
use tokio::sync::broadcast;

const RING: usize = 256;

/// One server-sent event, scoped to a household.
#[derive(Debug, Clone)]
pub struct HubEvent {
    pub id: u64,
    pub household: HouseholdId,
    pub kind: String,
    pub data: serde_json::Value,
}

struct Inner {
    next_id: u64,
    ring: VecDeque<HubEvent>,
}

/// In-process broadcast hub with a ring buffer of the last 256 events for `Last-Event-ID` replay.
#[derive(Clone)]
pub struct Hub {
    inner: Arc<Mutex<Inner>>,
    tx: broadcast::Sender<HubEvent>,
}

impl Default for Hub {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(RING);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                next_id: 1,
                ring: VecDeque::with_capacity(RING),
            })),
            tx,
        }
    }
}

impl Hub {
    /// Stamps the event with the next monotonic id, stores it and broadcasts it.
    pub fn publish(&self, household: HouseholdId, kind: &str, data: serde_json::Value) -> u64 {
        let mut g = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let id = g.next_id;
        g.next_id += 1;
        let ev = HubEvent {
            id,
            household,
            kind: kind.to_owned(),
            data,
        };
        if g.ring.len() == RING {
            g.ring.pop_front();
        }
        g.ring.push_back(ev.clone());
        // No subscribers is fine.
        let _ = self.tx.send(ev);
        id
    }

    /// Subscribes and returns the buffered events newer than `after` for `household`. Both happen
    /// under the lock, so no event is lost or duplicated between replay and live.
    pub fn subscribe(
        &self,
        household: HouseholdId,
        after: Option<u64>,
    ) -> (Vec<HubEvent>, broadcast::Receiver<HubEvent>) {
        let g = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let rx = self.tx.subscribe();
        let replay = match after {
            Some(a) => g
                .ring
                .iter()
                .filter(|e| e.id > a && e.household == household)
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        (replay, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_and_replay() {
        let hub = Hub::default();
        let h = HouseholdId::new();
        let other = HouseholdId::new();
        for i in 0..300 {
            hub.publish(
                if i % 2 == 0 { h } else { other },
                "x",
                serde_json::json!(i),
            );
        }
        let (replay, _rx) = hub.subscribe(h, Some(290));
        assert_eq!(
            replay.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![291, 293, 295, 297, 299]
        );
        let (old, _rx) = hub.subscribe(h, Some(0));
        assert!(old.len() <= RING && old.iter().all(|e| e.id > 44));
    }
}
