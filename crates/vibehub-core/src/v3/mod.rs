pub mod application;
pub mod domain;
pub mod event_store;
pub mod git_runner;
pub mod lifecycle;
pub mod orchestration;
pub mod project_intelligence;
pub mod projection;
pub mod views;
pub mod worktree;

pub use application::V3ApplicationService;
pub use domain::*;
pub use event_store::V3EventStore;
pub use git_runner::*;
pub use lifecycle::{fold_task as fold_task_lifecycle, LifecycleCommand, TaskLifecycleProjection};
pub use orchestration::{
    fold_task as fold_worktree_orchestration, LeaseProjection, LeaseState, OrchestrationCommand,
    OrchestrationProjection, WorktreeProjection,
};
pub use project_intelligence::{ProjectIndexService, ProjectModelSnapshot, ProjectPage};
pub use views::{V3ViewBundle, V3ViewRepository};
pub use worktree::*;
