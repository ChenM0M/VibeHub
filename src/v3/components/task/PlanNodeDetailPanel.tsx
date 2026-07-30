import { AlertTriangle, Loader2, RefreshCw } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { AcceptanceProgress } from "@/v3/components/task/AcceptanceProgress";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import { NodeBriefPanel } from "@/v3/components/task/NodeBriefPanel";
import type { PlanGraphView } from "@/v3/contracts/generated/plan-graph-view";
import type { CriterionSummary } from "@/v3/contracts/generated/project-overview-view";
import type { V3NodeBriefDetail } from "@/v3/stores/v3Store";
import { useTranslation } from "react-i18next";

type PlanNode = PlanGraphView["nodes"][number];

interface PlanNodeDetailPanelProps {
  node: PlanNode | null;
  dependencies: string[];
  criteria: CriterionSummary[];
  detail: V3NodeBriefDetail;
  onRetry?: () => void;
}

function FactList({ items, emptyLabel }: { items: string[]; emptyLabel: string }) {
  if (items.length === 0) return <p className="text-sm italic text-muted-foreground">{emptyLabel}</p>;
  return (
    <ul className="list-inside list-disc space-y-0.5 text-sm">
      {items.map((item) => <li key={item} className="break-all">{item}</li>)}
    </ul>
  );
}

function Fact({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border border-border/70 p-3">
      <div className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{title}</div>
      {children}
    </div>
  );
}

/**
 * Detail of one clicked plan node. The node-level facts come from the plan graph
 * itself, so every node - completed ones included - always renders something.
 * The task-level brief is fetched per node and its loading/unavailable states
 * are shown instead of leaving the click without an answer.
 */
export function PlanNodeDetailPanel({ node, dependencies, criteria, detail, onRetry }: PlanNodeDetailPanelProps) {
  const { t } = useTranslation();
  const showsOwnBrief = detail.status === "ready" && detail.brief !== null && detail.brief.node_id === detail.nodeId;

  return (
    <div className="space-y-4 p-6">
      <div className="flex items-start justify-between gap-4 border-b border-border/60 pb-3">
        <div className="min-w-0 flex-1">
          <h3 className="text-lg font-semibold">{node?.title ?? detail.nodeId}</h3>
          <div className="mt-1.5 flex flex-wrap items-center gap-2">
            {node && <Badge variant="outline">{t(`v3.common.nodeState.${node.state}`, { defaultValue: node.state })}</Badge>}
            {node && <Badge variant="outline">{t(`v3.plan.detail.readiness.${node.readiness}`, { defaultValue: node.readiness })}</Badge>}
            <span className="font-mono text-[11px] text-muted-foreground">{detail.nodeId}</span>
          </div>
        </div>
      </div>

      {node && <div className="space-y-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("v3.plan.detail.nodeFacts")}</div>
        <Fact title={t("v3.plan.detail.goal")}>
          <p className="text-sm">{node.goal || t("v3.common.none")}</p>
        </Fact>
        <div className="grid grid-cols-2 gap-3">
          <Fact title={t("v3.nodeBrief.scope")}><FactList items={node.scope} emptyLabel={t("v3.common.none")} /></Fact>
          <Fact title={t("v3.nodeBrief.dependencies")}><FactList items={dependencies} emptyLabel={t("v3.common.none")} /></Fact>
          <Fact title={t("v3.plan.detail.sessions")}><FactList items={node.session_ids ?? []} emptyLabel={t("v3.plan.noSessions")} /></Fact>
          <Fact title={t("v3.plan.detail.blockReasons")}><FactList items={node.block_reasons} emptyLabel={t("v3.common.none")} /></Fact>
        </div>
        <BlockerDetailsPanel blockers={node.blocker_details} />
        <Fact title={t("v3.plan.detail.criteria")}>
          {criteria.length === 0
            ? <p className="text-sm italic text-muted-foreground">{t("v3.plan.detail.noCriteria")}</p>
            : <AcceptanceProgress criteria={criteria} />}
        </Fact>
      </div>}

      <div className="border-t border-border/60 pt-4">
        <div className="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("v3.plan.detail.fullBrief")}</div>
        {detail.status === "loading" && (
          <div className="flex items-center gap-2 border border-border/70 p-4 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("v3.plan.detail.loading")}
          </div>
        )}
        {detail.status === "unavailable" && (
          <div role="status" className="space-y-2 border border-orange-500/30 bg-orange-500/5 p-4">
            <div className="flex items-center gap-2 text-sm font-medium text-orange-700 dark:text-orange-400">
              <AlertTriangle className="h-4 w-4" />
              {t("v3.plan.detail.unavailable")}
            </div>
            <p className="text-xs text-muted-foreground">{t("v3.plan.detail.unavailableHint")}</p>
            {detail.error && <pre className="whitespace-pre-wrap break-all bg-muted p-2 font-mono text-[11px]">{detail.error}</pre>}
            {onRetry && (
              <Button variant="outline" size="sm" onClick={onRetry}>
                <RefreshCw className="mr-1.5 h-3.5 w-3.5" />
                {t("v3.plan.detail.retry")}
              </Button>
            )}
          </div>
        )}
      </div>

      {showsOwnBrief && <div className="-mx-6 -mb-6"><NodeBriefPanel data={detail.brief!} /></div>}
    </div>
  );
}
