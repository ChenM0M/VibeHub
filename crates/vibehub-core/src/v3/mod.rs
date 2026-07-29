pub mod agent_specs;
pub mod application;
pub mod blockers;
pub mod bootstrap;
pub mod domain;
pub mod event_store;
pub mod execution_policy;
pub mod git_runner;
pub mod lifecycle;
pub mod orchestration;
pub mod project_intelligence;
pub mod project_memory;
pub mod project_scopes;
pub mod project_settings;
pub mod projection;
pub mod routing;
pub mod task_creation;
pub mod task_quarantine;
pub mod views;
pub mod worktree;

pub use agent_specs::*;
pub use application::V3ApplicationService;
pub use blockers::*;
pub use bootstrap::*;
pub use domain::*;
pub use event_store::V3EventStore;
pub use execution_policy::*;
pub use git_runner::*;
pub use lifecycle::{
    fold_task as fold_task_lifecycle, is_terminal_plan_node_state, plan_node_state_covers_criteria,
    LifecycleCommand, PlanAddNodeCommand, PlanCommandIdentity, PlanSetCriteriaCommand,
    PlanSetDependenciesCommand, PlanSetStateCommand, TaskLifecycleProjection,
};
pub use orchestration::{
    fold_task as fold_worktree_orchestration, LeaseProjection, LeaseState, OrchestrationCommand,
    OrchestrationProjection, WorktreeProjection,
};
pub use project_intelligence::{ProjectIndexService, ProjectModelSnapshot, ProjectPage};
pub use project_memory::*;
pub use project_scopes::*;
pub use project_settings::*;
pub use task_creation::{create_v3_task, V3TaskCreateRequest, V3TaskCreateResult};
pub use task_quarantine::{quarantine_v3_task, V3TaskQuarantineResult};
pub use views::{V3ViewBundle, V3ViewRepository};
pub use worktree::*;
