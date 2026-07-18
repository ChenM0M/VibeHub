import type { StructuredError, Warning } from "@/v3/contracts/generated/project-overview-view";

export type Diagnostic = Warning | StructuredError;

const diagnosticTranslationKeys: Record<string, string> = {
  "v3.warning.architecture_index_pending": "v3.diagnostics.messages.architectureIndexPending",
  "v3.warning.plan_not_recorded": "v3.diagnostics.messages.planNotRecorded",
  "v3.warning.task_metadata_invalid": "v3.diagnostics.messages.taskMetadataInvalid",
  "v3.warning.agent_result_session_closed": "v3.diagnostics.messages.agentResultSessionClosed",
  "v3.warning.agent_result_status_invalid": "v3.diagnostics.messages.agentResultStatusInvalid",
  "v3.warning.agent_result_details_normalized": "v3.diagnostics.messages.agentResultDetailsNormalized",
  "v3.warning.protocol_coverage_gap": "v3.diagnostics.messages.protocolCoverageGap",
  "v3.warning.project_uninitialized": "v3.diagnostics.messages.projectUninitialized",
  "v3.warning.model_stale": "v3.diagnostics.messages.modelStale",
  "v3.warning.scope_overlap": "v3.diagnostics.messages.scopeOverlap",
  "v3.warning.subtree_inaccessible": "v3.diagnostics.messages.subtreeInaccessible",
  "v3.warning.no_declared_architecture": "v3.diagnostics.messages.noDeclaredArchitecture",
  "v3.warning.project_index_gap": "v3.diagnostics.messages.projectIndexGap",
  "v3.warning.analyzer_degraded": "v3.diagnostics.messages.analyzerDegraded",
  "v3.warning.analyzer_unsupported": "v3.diagnostics.messages.analyzerUnsupported",
  "v3.warning.architecture_unsupported": "v3.diagnostics.messages.architectureUnsupported",
  "v3.error.project_index_unavailable": "v3.diagnostics.messages.projectIndexUnavailable",
  "v3.error.projection_unavailable": "v3.diagnostics.messages.projectionUnavailable",
  "v3.error.contract_terminal": "v3.diagnostics.messages.contractTerminal",
  "warning.protocol_coverage_gap": "v3.diagnostics.messages.protocolCoverageGap",
  "warning.project_uninitialized": "v3.diagnostics.messages.projectUninitialized",
  "warning.model_stale": "v3.diagnostics.messages.modelStale",
  "warning.scope_overlap": "v3.diagnostics.messages.scopeOverlap",
  "warning.subtree_inaccessible": "v3.diagnostics.messages.subtreeInaccessible",
  "warning.analyzer_unsupported": "v3.diagnostics.messages.analyzerUnsupported",
  "warning.no_declared_architecture": "v3.diagnostics.messages.noDeclaredArchitecture",
  "error.projection_unavailable": "v3.diagnostics.messages.projectionUnavailable",
  "error.contract_terminal": "v3.diagnostics.messages.contractTerminal",
};

const diagnosticCodeTranslationKeys: Record<string, string> = {
  PI_INDEX_UNAVAILABLE: "v3.diagnostics.messages.projectIndexUnavailable",
  PI_ANALYZER_DEGRADED: "v3.diagnostics.messages.analyzerDegraded",
  PI_ANALYZER_UNSUPPORTED: "v3.diagnostics.messages.analyzerUnsupported",
  PI_ARCHITECTURE_UNSUPPORTED: "v3.diagnostics.messages.architectureUnsupported",
};

const detailLabelKeys: Record<string, string> = {
  analyzer: "v3.diagnostics.details.analyzer",
  analyzers: "v3.diagnostics.details.analyzers",
  message: "v3.diagnostics.details.message",
  path: "v3.diagnostics.details.path",
};

export interface DiagnosticDetail {
  labelKey: string;
  labelParams?: Record<string, string>;
  value?: string;
  valueKey?: string;
  valueParams?: Record<string, string>;
}

export function diagnosticTranslationKey(messageKey: string, kind: "warning" | "error", code?: string): string {
  return (code && diagnosticCodeTranslationKeys[code]) ?? diagnosticTranslationKeys[messageKey] ?? `v3.diagnostics.${kind}.unknown`;
}

export function isKnownDiagnostic(messageKey: string, kind: "warning" | "error", code?: string): boolean {
  return Boolean((code && diagnosticCodeTranslationKeys[code]) ?? diagnosticTranslationKeys[messageKey]) || diagnosticTranslationKey(messageKey, kind, code) !== `v3.diagnostics.${kind}.unknown`;
}

function formatDetailValue(value: unknown): string {
  if (Array.isArray(value)) return value.map(formatDetailValue).join("、");
  if (value && typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function rawDetailEntries(details: Record<string, unknown> | undefined): DiagnosticDetail[] {
  return Object.entries(details ?? {}).map(([field, value]) => ({
    labelKey: detailLabelKeys[field] ?? "v3.diagnostics.details.rawField",
    labelParams: detailLabelKeys[field] ? undefined : { field: field.replace(/_/g, " ") },
    value: formatDetailValue(value),
  }));
}

export function diagnosticDetailEntries(diagnostic: Diagnostic): DiagnosticDetail[] {
  const details = diagnostic.details ?? {};
  const raw = rawDetailEntries(details);

  switch (diagnostic.code) {
    case "PI_ANALYZER_UNSUPPORTED": {
      const analyzers = details.analyzers;
      return [
        ...(analyzers ? [{ labelKey: "v3.diagnostics.details.analyzers", value: formatDetailValue(analyzers) }] : []),
        { labelKey: "v3.diagnostics.details.impact", valueKey: "v3.diagnostics.messages.analyzerUnsupportedImpact" },
        { labelKey: "v3.diagnostics.details.nextStep", valueKey: "v3.diagnostics.messages.analyzerUnsupportedNextStep" },
      ];
    }
    case "PI_ANALYZER_DEGRADED":
      return [
        { labelKey: "v3.diagnostics.details.impact", valueKey: "v3.diagnostics.messages.analyzerDegradedImpact" },
        { labelKey: "v3.diagnostics.details.nextStep", valueKey: "v3.diagnostics.messages.analyzerDegradedNextStep" },
        ...raw,
      ];
    case "PI_ARCHITECTURE_UNSUPPORTED":
      return [
        { labelKey: "v3.diagnostics.details.impact", valueKey: "v3.diagnostics.messages.architectureUnsupportedImpact" },
        { labelKey: "v3.diagnostics.details.nextStep", valueKey: "v3.diagnostics.messages.architectureUnsupportedNextStep" },
        ...raw,
      ];
    default:
      return raw;
  }
}
