// ===== TaskId =====

#[derive(Debug, Clone, Copy)]
pub struct TaskId(u32);

impl TaskId {
    pub fn from_idx(idx: usize) -> Self {
        Self(idx.try_into().unwrap())
    }

    pub fn to_idx(self) -> usize {
        self.0 as usize
    }
}
