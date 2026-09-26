//! Per-thread instrumentation used by deterministic query benchmarks.
use serde::Serialize;
use std::cell::RefCell;
#[derive(Debug, Clone, Default, Serialize)]
pub struct ReadWork {
    pub bundles: u64,
    pub task_history_reads: u64,
    pub project_history_reads: u64,
    pub decoded_events: u64,
}
thread_local! {static WORK:RefCell<ReadWork>=RefCell::new(ReadWork::default());}
pub fn reset() {
    WORK.with(|w| *w.borrow_mut() = ReadWork::default());
}
pub fn snapshot() -> ReadWork {
    WORK.with(|w| w.borrow().clone())
}
pub(crate) fn bundle() {
    WORK.with(|w| w.borrow_mut().bundles += 1);
}
pub(crate) fn task_history() {
    WORK.with(|w| w.borrow_mut().task_history_reads += 1);
}
pub(crate) fn project_history() {
    WORK.with(|w| w.borrow_mut().project_history_reads += 1);
}
pub(crate) fn events(n: usize) {
    WORK.with(|w| w.borrow_mut().decoded_events += n as u64);
}
