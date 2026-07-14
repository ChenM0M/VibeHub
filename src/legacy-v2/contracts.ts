export type LegacyV2SourceState = "absent" | "available" | "degraded";

export interface LegacyV2Card {
  task_id: string;
  title: string;
  state: string | null;
  completed_at: string | null;
  phase: string | null;
  final_summary: string | null;
  file_links: string[];
  warnings: string[];
}

export interface LegacyV2Archive {
  source_state: LegacyV2SourceState;
  cards: LegacyV2Card[];
  warnings: string[];
}
