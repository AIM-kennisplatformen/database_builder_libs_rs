use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ProgressEvent {
    FileStarted {
        worker_id: usize,
        file_path: String,
        total_steps: usize,
        message: String,
    },
    Step {
        worker_id: usize,
        step: usize,
        message: Option<String>,
    },
    FileFinished {
        worker_id: usize,
    },
}

pub trait Progress: Send + Sync {
    fn report(&self, event: ProgressEvent);
}
