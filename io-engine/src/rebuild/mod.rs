mod rebuild_descriptor;
mod rebuild_error;
mod rebuild_job;
mod rebuild_job_backend;
mod rebuild_record;
mod rebuild_state;
mod rebuild_task;

use rebuild_descriptor::RebuildDescriptor;
pub use rebuild_error::RebuildError;
pub use rebuild_job::RebuildJob;
use rebuild_job::RebuildOperation;
use rebuild_job_backend::{
    RebuildFBendChan,
    RebuildJobBackend,
    RebuildJobRequest,
};
pub use rebuild_record::RebuildRecord;
pub use rebuild_state::RebuildState;
use rebuild_state::RebuildStates;
use rebuild_task::{RebuildTask, RebuildTasks, TaskResult};

/// Number of concurrent copy tasks per rebuild job
const SEGMENT_TASKS: usize = 16;

/// Size of each segment used by the copy task
pub const SEGMENT_SIZE: u64 =
    spdk_rs::libspdk::SPDK_BDEV_LARGE_BUF_MAX_SIZE as u64;

/// Checks whether a range is contained within another range
pub trait Within<T> {
    /// True if `self` is contained within `right`, otherwise false
    fn within(&self, right: std::ops::Range<T>) -> bool;
}

impl Within<u64> for std::ops::Range<u64> {
    fn within(&self, right: std::ops::Range<u64>) -> bool {
        // also make sure ranges don't overflow
        self.start < self.end
            && right.start < right.end
            && self.start >= right.start
            && self.end <= right.end
    }
}

/// Rebuild statistics.
#[derive(Default, Debug, Clone)]
pub struct RebuildStats {
    /// Total number of blocks to recover.
    pub blocks_total: u64,
    /// Number of blocks recovered.
    pub blocks_recovered: u64,
    /// Rebuild progress in %.
    pub progress: u64,
    /// Granularity of each recovery copy in blocks.
    pub segment_size_blks: u64,
    /// Size in bytes of each block.
    pub block_size: u64,
    /// Total number of concurrent rebuild tasks.
    pub tasks_total: u64,
    /// Number of current active tasks.
    pub tasks_active: u64,
}
