use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolProgress {
    pub tool_name: String,
    pub tool_id: String,
    pub progress: f64, // 0.0 to 1.0
    pub message: Option<String>,
    pub timestamp: u64,
}

pub struct ToolProgressTracker {
    progress: Arc<Mutex<HashMap<String, ToolProgress>>>,
    sender: broadcast::Sender<ToolProgress>,
}

impl ToolProgressTracker {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            progress: Arc::new(Mutex::new(HashMap::new())),
            sender,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ToolProgress> {
        self.sender.subscribe()
    }

    pub async fn update_progress(&self, progress: ToolProgress) {
        let mut progress_map = self.progress.lock().await;
        progress_map.insert(progress.tool_id.clone(), progress.clone());
        let _ = self.sender.send(progress);
    }

    pub async fn get_progress(&self, tool_id: &str) -> Option<ToolProgress> {
        let progress_map = self.progress.lock().await;
        progress_map.get(tool_id).cloned()
    }
}
