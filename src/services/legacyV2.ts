import { invoke } from "@tauri-apps/api/core";
import type { LegacyV2Archive } from "@/legacy-v2/contracts";

export type LegacyV2Loader = (projectPath: string) => Promise<LegacyV2Archive>;

export const loadLegacyV2Archive: LegacyV2Loader = (projectPath) =>
  invoke<LegacyV2Archive>("legacy_v2_load_archive", { projectPath });
