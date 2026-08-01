use super::{
    cpu::CpuRef,
    state::State,
    task::{Task, TaskExecutionAuthority, TaskRef},
    task_flow::{TaskFlow, TaskFlowRef},
};

/// A short-lived proof that the architecture's current Task identity and the
/// effective TaskFlow agree. The capability owns no Task state and permits no
/// mutation; callers retain only the generation-checked typed reference.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CurrentTask {
    task_ref: TaskRef,
}

impl CurrentTask {
    pub(crate) const fn new(task_ref: TaskRef) -> Self {
        Self { task_ref }
    }

    pub const fn task_ref(self) -> TaskRef {
        self.task_ref
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CurrentTaskErrorCode {
    UnknownIdentity,
    StaleTaskRef,
    NotOnCpuLive,
    MissingActiveFlow,
    FlowMismatch,
    MissingCpu,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CurrentTaskDiagnostic {
    tp: usize,
    task_ref: TaskRef,
    flow_ref: TaskFlowRef,
    cpu_ref: CpuRef,
}

impl CurrentTaskDiagnostic {
    pub(crate) const fn new(
        tp: usize,
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
    ) -> Self {
        Self {
            tp,
            task_ref,
            flow_ref,
            cpu_ref,
        }
    }

    pub const fn tp(self) -> usize {
        self.tp
    }

    pub const fn task_ref(self) -> TaskRef {
        self.task_ref
    }

    pub const fn flow_ref(self) -> TaskFlowRef {
        self.flow_ref
    }

    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CurrentTaskError {
    code: CurrentTaskErrorCode,
    diagnostic: CurrentTaskDiagnostic,
}

impl CurrentTaskError {
    pub(crate) const fn new(code: CurrentTaskErrorCode, diagnostic: CurrentTaskDiagnostic) -> Self {
        Self { code, diagnostic }
    }

    pub const fn code(self) -> CurrentTaskErrorCode {
        self.code
    }

    pub const fn diagnostic(self) -> CurrentTaskDiagnostic {
        self.diagnostic
    }
}

/// The canonical Task/TaskFlow pair located by the centralized identity
/// resolver. It is kept crate-private so ordinary callers never receive a raw
/// address or a mutable Task borrow.
pub(crate) struct CurrentTaskCandidate<'a> {
    pub(crate) task: &'a Task,
    pub(crate) flow: &'a TaskFlow,
}

pub(crate) fn validate_candidate(
    tp: usize,
    requested_ref: TaskRef,
    candidate: CurrentTaskCandidate<'_>,
) -> Result<CurrentTask, CurrentTaskError> {
    let task_ref = candidate.task.task_ref();
    let flow_ref = candidate.flow.flow_ref();
    let cpu_ref = candidate.flow.cpu_ref().unwrap_or(CpuRef::invalid());
    let diagnostic = CurrentTaskDiagnostic::new(tp, task_ref, flow_ref, cpu_ref);

    if !requested_ref.is_valid() || !task_ref.same_identity(requested_ref) {
        return Err(CurrentTaskError::new(
            CurrentTaskErrorCode::StaleTaskRef,
            diagnostic,
        ));
    }
    if candidate.task.state() != State::OnCpu
        || candidate.task.execution_authority() != TaskExecutionAuthority::Live
    {
        return Err(CurrentTaskError::new(
            CurrentTaskErrorCode::NotOnCpuLive,
            diagnostic,
        ));
    }
    let fixed_flow_matches = candidate.task.flow().same_identity(flow_ref)
        && matches!(
            candidate.flow.state(),
            State::Base | State::Prepared | State::Ready | State::Online
        );
    if !fixed_flow_matches {
        return Err(CurrentTaskError::new(
            CurrentTaskErrorCode::MissingActiveFlow,
            diagnostic,
        ));
    }
    if !candidate.task.owns_flow(flow_ref)
        || !candidate.flow.declared()
        || !candidate.flow.owner().same_identity(task_ref)
    {
        return Err(CurrentTaskError::new(
            CurrentTaskErrorCode::FlowMismatch,
            diagnostic,
        ));
    }
    if !cpu_ref.is_valid() {
        return Err(CurrentTaskError::new(
            CurrentTaskErrorCode::MissingCpu,
            diagnostic,
        ));
    }
    Ok(CurrentTask::new(task_ref))
}
