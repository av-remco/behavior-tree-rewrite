

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
pub enum Status {
    Success,
    Failure,
    Running,
    Idle,
}

impl Status {
    pub fn is_running(&self) -> bool {
        matches!(self, Status::Running)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Status::Failure)
    }

    pub fn is_idle(&self) -> bool {
        matches!(self, Status::Idle)
    }

    pub fn is_succes(&self) -> bool {
        matches!(self, Status::Success)
    }
}