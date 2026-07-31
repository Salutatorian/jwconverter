//! In-memory active conversion registry and batch queue state.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::engine::image_queue::ImageQueueState;
use crate::engine::link_queue::LinkQueueState;
use crate::engine::queue::QueueState;
use crate::engine::runner::ActiveProcess;
use crate::media::ffmpeg;

#[derive(Default)]
pub struct AppState {
    pub active: Mutex<HashMap<String, ActiveProcess>>,
    pub link_queue: Mutex<LinkQueueState>,
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

    pub fn remove(&self, job_id: &str) {
        if let Ok(mut map) = self.active.lock() {
            map.remove(job_id);
        }
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
        if let Ok(queue) = self.link_queue.lock() {
            queue
                .cancel_remaining
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
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
        let link_queue = state.link_queue.lock().expect("link queue lock");
        assert!(!link_queue.worker_running);
        assert_eq!(link_queue.parallelism, 2);
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
