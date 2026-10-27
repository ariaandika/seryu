use core::ops;

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

// ===== TaskState =====

/// Task state.
#[derive(Clone, Copy)]
pub struct Task(u32);

impl Task {
    pub const EMPTY: Self = Self(0);
    pub const RD: Self = Self(1 << 0);
    pub const QUEUE: Self = Self(1 << 1);
    pub const WR: Self = Self(1 << 2);

    pub fn has(&self, state: Task) -> bool {
        self.0 & state.0 == state.0
    }

    pub fn unset(&self, state: Task) -> Self {
        Self(self.0 & (!state.0))
    }
}

impl ops::BitOr for Task {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0.bitor(rhs.0))
    }
}

impl ops::BitOrAssign for Task {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0.bitor_assign(rhs.0);
    }
}

impl ops::BitAnd for Task {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0.bitand(rhs.0))
    }
}
