//! Offline Queue for buffering events when disconnected
//!
//! Uses SQLite for persistent storage of events that couldn't be sent.

use crate::error::OximyResult;
use oisp_core::events::OispEvent;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Offline queue for event buffering
///
/// Stores events in SQLite when the network is unavailable,
/// and allows retrieval for retry when connectivity is restored.
pub struct OfflineQueue {
    conn: Arc<Mutex<Connection>>,
    max_events: usize,
}

impl OfflineQueue {
    /// Create a new offline queue
    pub fn new(path: &str, max_events: usize) -> OximyResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Create table if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS offline_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_json TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                retry_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        // Create index for efficient retrieval
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_offline_events_created
             ON offline_events(created_at)",
            [],
        )?;

        info!("Offline queue initialized at {}", path);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            max_events,
        })
    }

    /// Create an in-memory queue (for testing)
    pub fn in_memory(max_events: usize) -> OximyResult<Self> {
        let conn = Connection::open_in_memory()?;

        conn.execute(
            "CREATE TABLE offline_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_json TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                retry_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            max_events,
        })
    }

    /// Enqueue events for later retry
    pub fn enqueue(&self, events: &[OispEvent]) -> OximyResult<()> {
        if events.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();

        // Check current count and enforce limit
        let current_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM offline_events", [], |row| row.get(0))?;

        let available_space = self.max_events.saturating_sub(current_count as usize);
        if available_space == 0 {
            warn!(
                "Offline queue full ({} events), dropping oldest",
                self.max_events
            );
            // Delete oldest events to make room
            let to_delete = events.len().min(self.max_events / 10).max(1);
            conn.execute(
                "DELETE FROM offline_events WHERE id IN
                 (SELECT id FROM offline_events ORDER BY created_at ASC LIMIT ?)",
                params![to_delete],
            )?;
        }

        // Insert events
        let mut stmt = conn.prepare(
            "INSERT INTO offline_events (event_json, created_at) VALUES (?, strftime('%s', 'now'))",
        )?;

        let events_to_insert = events
            .len()
            .min(available_space.max(events.len().min(self.max_events / 10)));
        for event in events.iter().take(events_to_insert) {
            let json = serde_json::to_string(event)?;
            stmt.execute(params![json])?;
        }

        debug!("Enqueued {} events to offline queue", events_to_insert);
        Ok(())
    }

    /// Dequeue events for retry (FIFO)
    pub fn dequeue(&self, limit: usize) -> OximyResult<Vec<OispEvent>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, event_json FROM offline_events
             ORDER BY created_at ASC LIMIT ?",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            let id: i64 = row.get(0)?;
            let json: String = row.get(1)?;
            Ok((id, json))
        })?;

        let mut events = Vec::new();
        let mut ids_to_delete = Vec::new();

        for row in rows {
            let (id, json) = row?;
            match serde_json::from_str::<OispEvent>(&json) {
                Ok(event) => {
                    events.push(event);
                    ids_to_delete.push(id);
                }
                Err(e) => {
                    warn!("Failed to deserialize queued event: {}", e);
                    ids_to_delete.push(id); // Delete corrupt events
                }
            }
        }

        // Delete dequeued events
        if !ids_to_delete.is_empty() {
            let placeholders: String = ids_to_delete
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("DELETE FROM offline_events WHERE id IN ({})", placeholders);
            let mut stmt = conn.prepare(&sql)?;

            for (i, id) in ids_to_delete.iter().enumerate() {
                stmt.raw_bind_parameter(i + 1, *id)?;
            }
            stmt.raw_execute()?;
        }

        debug!("Dequeued {} events from offline queue", events.len());
        Ok(events)
    }

    /// Peek at events without removing them
    pub fn peek(&self, limit: usize) -> OximyResult<Vec<OispEvent>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT event_json FROM offline_events
             ORDER BY created_at ASC LIMIT ?",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut events = Vec::new();
        for row in rows {
            let json = row?;
            if let Ok(event) = serde_json::from_str::<OispEvent>(&json) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get count of pending events
    pub fn pending_count(&self) -> OximyResult<usize> {
        let conn = self.conn.lock();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM offline_events", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Clear all queued events
    pub fn clear(&self) -> OximyResult<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM offline_events", [])?;
        info!("Offline queue cleared");
        Ok(())
    }

    /// Remove events older than given seconds
    pub fn cleanup_old(&self, max_age_secs: i64) -> OximyResult<usize> {
        let conn = self.conn.lock();
        let deleted = conn.execute(
            "DELETE FROM offline_events
             WHERE created_at < strftime('%s', 'now') - ?",
            params![max_age_secs],
        )?;

        if deleted > 0 {
            info!("Cleaned up {} old events from offline queue", deleted);
        }

        Ok(deleted)
    }

    /// Get queue statistics
    pub fn stats(&self) -> OximyResult<QueueStats> {
        let conn = self.conn.lock();

        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM offline_events", [], |row| row.get(0))?;

        let oldest: Option<i64> = conn
            .query_row("SELECT MIN(created_at) FROM offline_events", [], |row| {
                row.get(0)
            })
            .ok();

        let newest: Option<i64> = conn
            .query_row("SELECT MAX(created_at) FROM offline_events", [], |row| {
                row.get(0)
            })
            .ok();

        Ok(QueueStats {
            pending_count: count as usize,
            max_events: self.max_events,
            oldest_timestamp: oldest,
            newest_timestamp: newest,
        })
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    /// Number of pending events
    pub pending_count: usize,

    /// Maximum events allowed
    pub max_events: usize,

    /// Oldest event timestamp (Unix seconds)
    pub oldest_timestamp: Option<i64>,

    /// Newest event timestamp (Unix seconds)
    pub newest_timestamp: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use oisp_core::events::{AiRequestData, AiRequestEvent, EventEnvelope, OispEvent};

    fn test_event(id: &str) -> OispEvent {
        let mut envelope = EventEnvelope::new("ai.request");
        envelope.event_id = id.to_string();

        OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: AiRequestData {
                request_id: format!("req_{}", id),
                provider: None,
                model: None,
                auth: None,
                request_type: None,
                streaming: None,
                messages: vec![],
                messages_count: None,
                has_system_prompt: None,
                system_prompt_hash: None,
                tools: vec![],
                tools_count: None,
                tool_choice: None,
                parameters: None,
                has_rag_context: None,
                has_images: None,
                image_count: None,
                estimated_tokens: None,
                conversation: None,
                agent: None,
            },
        })
    }

    fn get_event_id(event: &OispEvent) -> &str {
        &event.envelope().event_id
    }

    #[test]
    fn test_enqueue_dequeue() {
        let queue = OfflineQueue::in_memory(1000).unwrap();

        let events = vec![test_event("1"), test_event("2"), test_event("3")];

        queue.enqueue(&events).unwrap();
        assert_eq!(queue.pending_count().unwrap(), 3);

        let dequeued = queue.dequeue(2).unwrap();
        assert_eq!(dequeued.len(), 2);
        assert_eq!(get_event_id(&dequeued[0]), "1");
        assert_eq!(get_event_id(&dequeued[1]), "2");

        assert_eq!(queue.pending_count().unwrap(), 1);

        let remaining = queue.dequeue(10).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(get_event_id(&remaining[0]), "3");

        assert_eq!(queue.pending_count().unwrap(), 0);
    }

    #[test]
    fn test_peek() {
        let queue = OfflineQueue::in_memory(1000).unwrap();

        let events = vec![test_event("1"), test_event("2")];
        queue.enqueue(&events).unwrap();

        let peeked = queue.peek(1).unwrap();
        assert_eq!(peeked.len(), 1);
        assert_eq!(get_event_id(&peeked[0]), "1");

        // Peek doesn't remove
        assert_eq!(queue.pending_count().unwrap(), 2);
    }

    #[test]
    fn test_clear() {
        let queue = OfflineQueue::in_memory(1000).unwrap();

        let events = vec![test_event("1"), test_event("2"), test_event("3")];
        queue.enqueue(&events).unwrap();
        assert_eq!(queue.pending_count().unwrap(), 3);

        queue.clear().unwrap();
        assert_eq!(queue.pending_count().unwrap(), 0);
    }

    #[test]
    fn test_max_events_limit() {
        let queue = OfflineQueue::in_memory(5).unwrap();

        // Enqueue 10 events, but max is 5
        let events: Vec<_> = (0..10).map(|i| test_event(&i.to_string())).collect();
        queue.enqueue(&events).unwrap();

        // Should have at most max_events
        let count = queue.pending_count().unwrap();
        assert!(count <= 5, "Count {} exceeds max 5", count);
    }

    #[test]
    fn test_stats() {
        let queue = OfflineQueue::in_memory(1000).unwrap();

        let stats = queue.stats().unwrap();
        assert_eq!(stats.pending_count, 0);
        assert_eq!(stats.max_events, 1000);
        assert!(stats.oldest_timestamp.is_none());

        let events = vec![test_event("1")];
        queue.enqueue(&events).unwrap();

        let stats = queue.stats().unwrap();
        assert_eq!(stats.pending_count, 1);
        assert!(stats.oldest_timestamp.is_some());
    }
}
