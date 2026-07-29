import type { V3AgentSpecTarget, V3OutputLanguage } from "@/v3/contracts";

export interface ProjectSetupInitialValues {
  language: V3OutputLanguage;
  gitUrl: string;
  tools: readonly V3AgentSpecTarget[];
  revision: number;
}

export function projectSetupSignature(values: ProjectSetupInitialValues): string {
  return [values.language, values.gitUrl, String(values.revision), [...values.tools].sort().join(",")].join("|");
}

export function shouldAdoptProjectSetupValues(options: { nextSignature: string; syncedSignature: string; dirty: boolean }): boolean {
  if (options.nextSignature === options.syncedSignature) return false;
  return !options.dirty;
}
