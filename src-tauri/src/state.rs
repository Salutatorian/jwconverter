//! In-memory active conversion registry and batch queue state.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::engine::image_queue::ImageQueueState;
use crate::engine::queue::QueueState;
use crate::engine::runner::ActiveProcess;
use crate::media::ffmpeg;

#[derive(Default)]
pub struct AppState {
    pub active: Mutex<HashMap<String, ActiveProcess>>,
    /// At most one experimental Links download at a time.
    pub active_link_job: Mutex<Option<String>>,
    pub queue: Mutex<QueueState>,
    pub image_queue: Mutex<ImageQueueState>,
}

impl AppState {
    pub fn register(&self, job_id: String) -> ActiveProcess {
        let active = ActiveProcess {
            child: Arc::new(Mutex::new(None)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        };
        if let Ok(mut map) = self.active.lock() {
            map.insert(
                job_id,
                ActiveProcess {
                    child: Arc::clone(&active.child),
                    cancel_flag: Arc::clone(&active.cancel_flag),
                },
            );
        }
        active
    }

    pub fn try_begin_link_job(&self, job_id: &str) -> bool {
        let Ok(mut slot) = self.active_link_job.lock() else {
            return false;
        };
        if slot.is_some() {
            return false;
        }
        *slot = Some(job_id.to_string());
        true
    }

    pub fn end_link_job(&self, job_id: &str) {
        if let Ok(mut slot) = self.active_link_job.lock() {
            if slot.as_deref() == Some(job_id) {
                *slot = None;
            }
        }
    }

    pub fn remove(&self, job_id: &str) {
        if let Ok(mut map) = self.active.lock() {
            map.remove(job_id);
        }
        self.end_link_job(job_id);
    }

    pub fn request_cancel(&self, job_id: &str) -> bool {
        let Ok(map) = self.active.lock() else {
            return false;
        };
        let Some(active) = map.get(job_id) else {
            return false;
        };
        active
            .cancel_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        ffmpeg::kill_child(&active.child);
        true
    }

    /// Cancel every active process (used on app shutdown).
    pub fn cancel_all(&self) {
        let Ok(map) = self.active.lock() else {
            return;
        };
        for active in map.values() {
            active
                .cancel_flag
                .store(true, std::sync::atomic::Ordering::SeqCst);
            ffmpeg::kill_child(&active.child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn register_tracks_active_process() {
        let state = AppState::default();
        let active = state.register("job-1".to_string());
        assert!(!active.cancel_flag.load(Ordering::SeqCst));

        // A second lookup through request_cancel sees the same flag object.
        assert!(state.request_cancel("job-1"));
        assert!(active.cancel_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn request_cancel_unknown_job_is_false() {
        let state = AppState::default();
        assert!(!state.request_cancel("nope"));
    }

    #[test]
    fn remove_forgets_registration() {
        let state = AppState::default();
        state.register("job-2".to_string());
        state.remove("job-2");
        assert!(!state.request_cancel("job-2"));
    }

    #[test]
    fn default_state_has_empty_queues() {
        let state = AppState::default();
        let queue = state.queue.lock().expect("queue lock");
        assert!(queue.items.is_empty());
        assert!(!queue.worker_running);
        drop(queue);
        let image_queue = state.image_queue.lock().expect("image queue lock");
        assert!(!image_queue.worker_running);
        drop(image_queue);
        assert!(state
            .active_link_job
            .lock()
            .expect("link lock")
            .is_none());
    }

    #[test]
    fn only_one_link_job_at_a_time() {
        let state = AppState::default();
        assert!(state.try_begin_link_job("a"));
        assert!(!state.try_begin_link_job("b"));
        state.end_link_job("a");
        assert!(state.try_begin_link_job("b"));
    }

    #[test]
    fn cancel_all_sets_flags() {
        let state = AppState::default();
        let a = state.register("job-a".into());
        let b = state.register("job-b".into());
        state.cancel_all();
        assert!(a.cancel_flag.load(Ordering::SeqCst));
        assert!(b.cancel_flag.load(Ordering::SeqCst));
    }
}
