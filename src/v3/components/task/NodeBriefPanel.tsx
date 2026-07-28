import { StateBadge } from "@/v3/components/common/StateBadge";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { NativePathDisplay } from "@/v3/components/common/NativePathDisplay";
import { AcceptanceProgress } from "@/v3/components/task/AcceptanceProgress";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import { Badge } from "@/components/ui/badge";
import type { NodeBrief } from "@/v3/contracts/generated/node-brief";
import { useTranslation } from "react-i18next";

function SubSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border border-border/70 p-4">
      <div className="mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wide">{title}</div>
      {children}
    </div>
  );
}
function TextList({ items, muted, emptyLabel }: { items: string[]; muted?: boolean; emptyLabel: string }) {
  if (items.length === 0) return <p className="text-sm text-muted-foreground italic">{emptyLabel}</p>;
  return <ul className={"list-disc list-inside text-sm space-y-0.5 " + (muted ? "text-muted-foreground" : "")}>{items.map((s, i) => <li key={i}>{s}</li>)}</ul>;
}

interface NodeBriefPanelProps { data: NodeBrief; }

export function NodeBriefPanel({ data }: NodeBriefPanelProps) {
  const { t, i18n } = useTranslation();
  const budgetUsed = data.budget.max_tokens > 0 ? (data.budget.estimated_tokens / data.budget.max_tokens) * 100 : 0;
  return (
    <div className="space-y-4 p-6">
      {/* 标题区 */}
      <div className="flex items-start justify-between gap-4 pb-3 border-b border-border/60">
        <div className="min-w-0 flex-1">
          <h3 className="text-lg font-semibold">{data.goal}</h3>
          <div className="mt-1.5 flex items-center gap-2">
            <Badge variant="outline">{t(`v3.common.nodeState.${data.state}`, { defaultValue: data.state })}</Badge>
            <Badge variant="outline">{t(`v3.cockpit.workflow.${data.workflow_profile}`)}</Badge>
            <span className="font-mono text-[11px] text-muted-foreground">{data.node_id}</span>
          </div>
        </div>
        <StateBadge freshness={data.freshness} completeness={data.completeness} />
      </div>

      {data.next_intent && (
        <div className="border border-blue-500/20 bg-blue-500/5 px-3 py-2 text-sm">
          <span className="text-muted-foreground">{t("v3.nodeBrief.nextIntent")}</span>{data.next_intent}
        </div>
      )}

      <BlockerDetailsPanel blockers={data.blocker_details} />

      <div className="grid grid-cols-3 gap-2 border border-border/70 p-3 text-xs"><div><span className="text-muted-foreground">{t("v3.nodeBrief.milestonePolicy")}</span><div className="mt-1 font-medium">{t(`v3.nodeBrief.policy.${data.execution_policy.milestone_policy}`)}</div></div><div><span className="text-muted-foreground">{t("v3.nodeBrief.planningRequired")}</span><div className="mt-1 font-medium">{t(data.execution_policy.planning_required ? "common.yes" : "common.no")}</div></div><div><span className="text-muted-foreground">{t("v3.nodeBrief.reviewRequired")}</span><div className="mt-1 font-medium">{t(data.execution_policy.review_required ? "common.yes" : "common.no")}</div></div></div>

      <SubSection title="Trigger / Effective Policy">
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div><span className="text-muted-foreground">Recommended</span><div>{data.execution_policy.recommended_profile}</div></div>
          <div><span className="text-muted-foreground">Effective</span><div>{data.execution_policy.effective_profile}</div></div>
          <div><span className="text-muted-foreground">Policy</span><div>v{data.execution_policy.policy_version} · {data.execution_policy.enforcement_epoch || "legacy"}</div></div>
          <div><span className="text-muted-foreground">Coverage</span><div>{data.coverage_mode}</div></div>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">{data.execution_policy.trigger_reasons.map((reason) => <Badge key={reason} variant="secondary">{reason}</Badge>)}</div>
        {data.execution_policy.override_record && <pre className="mt-2 whitespace-pre-wrap rounded bg-muted p-2 text-xs">Override: {JSON.stringify(data.execution_policy.override_record, null, 2)}</pre>}
        {data.execution_policy.upgrade_history.length > 0 && <div className="mt-2 space-y-1 text-xs">{data.execution_policy.upgrade_history.map((entry, index) => <div key={index} className="rounded border border-border/60 p-2">{JSON.stringify(entry)}</div>)}</div>}
      </SubSection>

      <div className="grid grid-cols-2 gap-4">
        <SubSection title="Protocol Coverage / Repair">
          <div className="space-y-1 text-xs">{data.protocol_records.map((record) => <div key={record.record} className="flex items-center justify-between gap-2"><span>{record.record}</span><span className={record.status === "complete" ? "text-emerald-600" : "text-orange-600"}>{record.status}{record.repair_action ? ` · ${record.repair_action}` : ""}</span></div>)}</div>
        </SubSection>
        <SubSection title="Completion Gate">
          <div className="space-y-2 text-xs">{data.completion_gate.items.map((item) => <details key={item.gate} className="border-b border-border/60 pb-2 last:border-0" open={!item.passed}>
            <summary className="flex cursor-pointer items-center justify-between gap-2"><span>{item.gate}</span><span className={item.passed ? "text-emerald-600" : "text-orange-600"}>{item.passed ? "PASS" : "BLOCKED"}</span></summary>
            <div className="mt-1 space-y-1 text-muted-foreground"><p>{item.observed_state}</p>{item.missing_facts.map((fact) => <p key={fact}>- {fact}</p>)}{item.repair_actions.map((action) => <p key={action} className="text-foreground">{action}</p>)}</div>
          </details>)}</div>
        </SubSection>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <SubSection title={t("v3.nodeBrief.scope")}><TextList items={data.scope} emptyLabel={t("v3.common.none")} /></SubSection>
        <SubSection title={t("v3.nodeBrief.nonScope")}><TextList items={data.non_scope} muted emptyLabel={t("v3.common.none")} /></SubSection>
        <SubSection title={t("v3.nodeBrief.dependencies")}><TextList items={data.dependencies} emptyLabel={t("v3.common.none")} /></SubSection>
        <SubSection title={t("v3.nodeBrief.decisions")}><TextList items={data.accepted_decisions} emptyLabel={t("v3.common.none")} /></SubSection>
      </div>

      {data.research_summary.length > 0 && <SubSection title={t("v3.nodeBrief.research")}><TextList items={data.research_summary} emptyLabel={t("v3.common.none")} /></SubSection>}

      <SubSection title="Project Memory · Why Injected">
        {data.project_memory.length === 0 ? <p className="text-sm text-muted-foreground italic">No relevant, fresh, injectable memory.</p> : <div className="space-y-2">{data.project_memory.map((entry) => <div key={entry.entry_id} className="rounded border border-border/70 p-3 text-sm"><div className="flex flex-wrap items-center gap-1"><Badge>{entry.kind}</Badge><Badge variant="outline">r{entry.revision}</Badge><Badge variant="outline">{entry.status}</Badge><span className="ml-auto font-mono text-[10px] text-muted-foreground">{entry.entry_id}</span></div><p className="mt-2 whitespace-pre-wrap">{entry.content}</p><p className="mt-2 text-xs text-muted-foreground">Why: {entry.why_injected}</p><p className="mt-1 text-[10px] text-muted-foreground">{entry.security_boundary} · evidence: {entry.evidence_refs.join(", ") || "none"}</p></div>)}</div>}
      </SubSection>

      <SubSection title={t("v3.nodeBrief.files")}>
        {data.files.length === 0 ? <p className="text-sm text-muted-foreground italic">{t("v3.nodeBrief.noFiles")}</p> : (
          <div className="space-y-1">{data.files.map((file, i) => <div key={i}><NativePathDisplay path={file} showPlatform /></div>)}</div>
        )}
      </SubSection>

      <SubSection title={t("v3.nodeBrief.validationCommands")}>
        {data.validation_commands.length === 0 ? <p className="text-sm text-muted-foreground italic">{t("v3.nodeBrief.noValidationCommands")}</p> : (
          <div className="space-y-1">{data.validation_commands.map((cmd, i) => <pre key={i} className="bg-muted p-2 text-xs font-mono whitespace-pre-wrap break-all">{cmd}</pre>)}</div>
        )}
      </SubSection>

      <div className="grid grid-cols-2 gap-4">
        <SubSection title={t("v3.nodeBrief.budget")}>
          <div className="space-y-1.5 text-sm">
            <div className="flex justify-between"><span className="text-muted-foreground">{t("v3.nodeBrief.maximum")}</span><span>{data.budget.max_tokens.toLocaleString(i18n.resolvedLanguage)}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">{t("v3.nodeBrief.estimated")}</span><span>{t("v3.nodeBrief.estimatedValue", { value: data.budget.estimated_tokens.toLocaleString(i18n.resolvedLanguage), percent: budgetUsed.toFixed(0) })}</span></div>
            <div className="h-1.5 bg-muted"><div className="h-full bg-amber-500" style={{ width: `${Math.min(budgetUsed, 100)}%` }} /></div>
            {data.budget.truncated_sections.length > 0 && <div className="text-orange-600 text-xs">{t("v3.nodeBrief.truncated", { value: data.budget.truncated_sections.join(t("v3.common.listSeparator")) })}</div>}
          </div>
        </SubSection>
        {Object.keys(data.source_versions).length > 0 && (
          <SubSection title={t("v3.nodeBrief.sourceVersions")}>
            <div className="space-y-1 text-sm">{Object.entries(data.source_versions).map(([key, version]) => <div key={key} className="flex justify-between"><span className="text-muted-foreground">{key}</span><span className="font-mono text-xs">{version}</span></div>)}</div>
          </SubSection>
        )}
      </div>

      <div>
        <AcceptanceProgress criteria={data.criteria} />
      </div>

      <EvidenceLink evidenceRefs={data.evidence_refs} />
    </div>
  );
}
