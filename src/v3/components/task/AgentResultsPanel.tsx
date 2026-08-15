import { CheckCircle2, Clock3, FileText, SearchCheck, XCircle } from "lucide-react";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { AgentResultsView, EvidenceRef } from "@/v3/contracts/generated/agent-results-view";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";

type SafeEvaluation = {
  target: string;
  rubric: string[];
  verdict: string;
  findings: { title: string; detail: string; severity: string; evidence_refs: EvidenceRef[] }[];
};

function safeEvidenceRefs(value: unknown): EvidenceRef[] {
  if (!Array.isArray(value)) return [];
  const refs: EvidenceRef[] = [];
  for (const entry of value) {
    if (typeof entry === "string") {
      const kind = entry.startsWith("file:") ? "file" : entry.startsWith("evt.") ? "event" : "external";
      refs.push({ evidence_id: entry, kind, grade: "agent_reported", label_key: "v3.evidence.agent_result", locator: entry });
      continue;
    }
    if (!entry || typeof entry !== "object") continue;
    const candidate = entry as Partial<EvidenceRef>;
    if (typeof candidate.evidence_id !== "string" || typeof candidate.locator !== "string") continue;
    refs.push({
      evidence_id: candidate.evidence_id,
      kind: candidate.kind ?? "external",
      grade: candidate.grade ?? "agent_reported",
      label_key: candidate.label_key ?? "v3.evidence.agent_result",
      locator: candidate.locator,
      captured_at: candidate.captured_at,
      excerpt: candidate.excerpt,
    });
  }
  return refs;
}

function safeEvaluation(value: unknown, t: TFunction): SafeEvaluation | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Record<string, unknown>;
  const rubric = Array.isArray(candidate.rubric) ? candidate.rubric.filter((item): item is string => typeof item === "string") : [];
  const findings = Array.isArray(candidate.findings) ? candidate.findings.flatMap((entry) => {
    if (typeof entry === "string") return [{ title: entry, detail: entry, severity: "medium", evidence_refs: [] }];
    if (!entry || typeof entry !== "object") return [];
    const finding = entry as Record<string, unknown>;
    return [{
      title: typeof finding.title === "string" ? finding.title : t("v3.agentResults.unnamedFinding"),
      detail: typeof finding.detail === "string" ? finding.detail : t("v3.agentResults.noDetail"),
      severity: typeof finding.severity === "string" ? finding.severity : "medium",
      evidence_refs: safeEvidenceRefs(finding.evidence_refs),
    }];
  }) : [];
  return {
    target: typeof candidate.target === "string" ? candidate.target : t("v3.agentResults.noTarget"),
    rubric,
    verdict: typeof candidate.verdict === "string" ? candidate.verdict : "inconclusive",
    findings,
  };
}

const statusIcon = {
  pending: Clock3,
  running: Clock3,
  succeeded: CheckCircle2,
  failed: XCircle,
};

export function AgentResultsPanel({ data }: { data: AgentResultsView }) {
  const { t, i18n } = useTranslation();
  const copy = data.review_required
    ? { title: t("v3.agentResults.reviewRequired.title"), detail: t("v3.agentResults.reviewRequired.detail") }
    : { title: t(`v3.agentResults.state.${data.state}.title`, { defaultValue: data.state }), detail: t(`v3.agentResults.state.${data.state}.detail`, { defaultValue: data.next_action ?? "" }) };
  if (data.results.length === 0) {
    return (
      <div className="flex h-full min-h-44 flex-col items-center justify-center rounded-md border border-dashed border-border px-6 text-center">
        <Clock3 className="mb-3 h-7 w-7 text-muted-foreground/60" />
        <div className="text-sm font-semibold">{copy.title}</div>
        <p className="mt-1 max-w-sm text-xs leading-relaxed text-muted-foreground">{copy.detail}</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {data.results.map((result) => {
        const runtimeStatus = result.status as string;
        const Icon = statusIcon[runtimeStatus as keyof typeof statusIcon] ?? XCircle;
        const evaluation = safeEvaluation(result.evaluation, t);
        const artifacts = Array.isArray(result.artifacts) ? result.artifacts : [];
        const evidenceRefs = safeEvidenceRefs(result.evidence_refs);
        return (
          <article key={result.result_id} className="min-w-0 rounded-md border border-border/50 bg-card/40 p-4 [overflow-wrap:anywhere]">
            <div className="flex items-start gap-2">
              <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h4 className="text-sm font-semibold">{result.summary || t(`v3.agentResults.kind.${result.kind}.result`)}</h4>
                  <span className="rounded border border-border/60 px-1.5 py-0.5 text-[10px] text-muted-foreground">{t(`v3.agentResults.kind.${result.kind}.label`)}</span>
                  <span className="rounded border border-border/60 px-1.5 py-0.5 text-[10px] text-muted-foreground">{runtimeStatus}</span>
                </div>
                <div className="mt-1 text-[11px] text-muted-foreground">{t(`v3.agentResults.request.${result.request.source}`)}：{result.request.instruction}</div>
              </div>
              {result.completed_at && <time className="shrink-0 text-[10px] text-muted-foreground">{new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "short" }).format(new Date(result.completed_at))}</time>}
            </div>

            {result.body && <p className="mt-3 whitespace-pre-wrap text-sm leading-relaxed text-foreground/85">{result.body}</p>}

            {evaluation && (
              <section className="mt-3 rounded border border-border/40 bg-muted/10 p-3">
                <div className="flex items-center gap-2 text-xs font-semibold"><SearchCheck className="h-3.5 w-3.5" />{evaluation.target}<span className="ml-auto uppercase text-muted-foreground">{evaluation.verdict}</span></div>
                <div className="mt-2 text-[11px] text-muted-foreground">{t("v3.agentResults.rubric", { value: evaluation.rubric.length > 0 ? evaluation.rubric.join(t("v3.common.listSeparator")) : t("v3.agentResults.noRubric") })}</div>
                {evaluation.findings.length > 0 && <div className="mt-2 space-y-2">{evaluation.findings.map((finding, index) => <div key={`${result.result_id}-${index}`} className="border-l-2 border-border pl-2"><div className="text-xs font-medium">{finding.title} · {finding.severity}</div><p className="text-[11px] text-muted-foreground">{finding.detail}</p><EvidenceLink evidenceRefs={finding.evidence_refs} /></div>)}</div>}
              </section>
            )}

            {artifacts.length > 0 && <div className="mt-3 flex flex-wrap gap-2">{artifacts.map((rawArtifact, index) => {
              const artifact = rawArtifact as unknown;
              const label = typeof artifact === "string" ? artifact : artifact && typeof artifact === "object" ? String((artifact as { label?: unknown; kind?: unknown; locator?: unknown }).label ?? (artifact as { kind?: unknown }).kind ?? (artifact as { locator?: unknown }).locator ?? t("v3.agentResults.unnamedArtifact")) : t("v3.agentResults.unnamedArtifact");
              const path = artifact && typeof artifact === "object" && (artifact as { path?: unknown }).path && typeof (artifact as { path?: unknown }).path === "object" ? (artifact as { path: { display?: unknown } }).path : null;
              return <div key={`${result.result_id}-artifact-${index}`} className="inline-flex max-w-full min-w-0 items-center gap-1.5 rounded border border-border/50 px-2 py-1 text-[11px]"><FileText className="h-3 w-3 shrink-0" /><span className="min-w-0 [overflow-wrap:anywhere]">{label}</span>{typeof path?.display === "string" && <span className="min-w-0 max-w-48 truncate text-muted-foreground">{path.display}</span>}</div>;
            })}</div>}
            <div className="mt-3"><EvidenceLink evidenceRefs={evidenceRefs} /></div>
          </article>
        );
      })}
    </div>
  );
}
