import type { V3FixtureBundle } from "@/v3/contracts/fixtureRepository";

export function findArchivedTaskAfterClosure(bundle: V3FixtureBundle | null, taskId: string) {
  return bundle?.projectOverview.archived_tasks?.find((task) => task.task_id === taskId) ?? null;
}
