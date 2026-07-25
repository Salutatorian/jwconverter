//! In-memory active conversion registry and batch queue state.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::engine::queue::QueueState;
use crate::engine::runner::ActiveProcess;
use crate::media::ffmpeg;

#[derive(Default)]
pub struct AppState {
    pub active: Mutex<HashMap<String, ActiveProcess>>,
    pub queue: Mutex<QueueState>,
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
}
