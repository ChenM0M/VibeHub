import { useState } from "react";
import { AlertTriangle, Check, ChevronDown, ChevronRight, CircleHelp, Clipboard, ExternalLink, ShieldAlert } from "lucide-react";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { BlockerDetail } from "@/v3/contracts/generated/project-overview-view";
import { useTranslation } from "react-i18next";

interface BlockerDetailsPanelProps {
  blockers?: BlockerDetail[];
  compact?: boolean;
}

export function BlockerDetailsPanel({ blockers = [], compact = false }: BlockerDetailsPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const [copiedAction, setCopiedAction] = useState<string>();
  const { t } = useTranslation();
  if (blockers.length === 0) return null;

  return (
    <section
      className={compact ? "space-y-2" : "rounded-md border border-orange-500/30 bg-orange-500/5"}
      aria-label={t("v3.blockers.title")}
    >
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
        className={compact ? "flex w-full min-w-0 items-center gap-2 text-left" : "flex w-full min-w-0 items-center gap-2 p-3 text-left hover:bg-orange-500/5"}
      >
        <ShieldAlert className="h-4 w-4 text-orange-600 dark:text-orange-400" />
        <h3 className="text-sm font-semibold text-orange-800 dark:text-orange-300">{t("v3.blockers.title")}</h3>
        <span className="rounded border border-orange-500/30 px-1.5 py-0.5 text-[10px] text-orange-700 dark:text-orange-300">
          {t("v3.common.itemCount", { count: blockers.length })}
        </span>
        {!expanded && <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">{blockers[0].summary}</span>}
        {expanded ? <ChevronDown className="ml-auto h-4 w-4 shrink-0 text-muted-foreground" /> : <ChevronRight className="ml-auto h-4 w-4 shrink-0 text-muted-foreground" />}
      </button>

      {expanded && <div className={compact ? "space-y-2" : "space-y-2 px-3 pb-3"}>
        {blockers.map((blocker) => (
          <article key={blocker.blocker_id} className="rounded border border-orange-500/20 bg-background/70 p-3">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-orange-600 dark:text-orange-400" />
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium leading-snug">{blocker.summary}</div>
                <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                  <span>{t("v3.blockers.type", { value: t(`v3.blockers.kind.${blocker.kind}`, { defaultValue: blocker.kind }) })}</span>
                  <span>{t("v3.blockers.owner", { value: blocker.owner })}</span>
                  <span className="font-mono text-[10px]">{blocker.reason_code}</span>
                </div>
              </div>
            </div>

             <div className="mt-2 grid gap-2 text-xs sm:grid-cols-2">
               <div className="border-l-2 border-orange-500/40 pl-2">
                 <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{t("v3.blockers.whyBlocked")}</div>
                 <div className="mt-0.5 leading-relaxed">{blocker.why_blocked}</div>
               </div>
               <div className="border-l-2 border-blue-500/40 pl-2">
                 <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{t("v3.blockers.stateDifference")}</div>
                 <div className="mt-0.5 leading-relaxed"><span className="text-muted-foreground">{t("v3.blockers.expected")}</span>{blocker.expected_state}</div>
                 <div className="mt-0.5 leading-relaxed"><span className="text-muted-foreground">{t("v3.blockers.observed")}</span>{blocker.observed_state}</div>
               </div>
             </div>

             {blocker.missing_facts.length > 0 && <div className="mt-2 text-xs">
               <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{t("v3.blockers.missingFacts")}</div>
               <ul className="mt-1 list-inside list-disc space-y-0.5">{blocker.missing_facts.map((fact) => <li key={fact}>{fact}</li>)}</ul>
             </div>}

             <div className="mt-2 space-y-2">
               <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{t("v3.blockers.nextSteps")}</div>
               {blocker.repair_actions.map((action) => <div key={action.action_id} className="rounded bg-blue-500/5 p-2 text-xs">
                 <div className="flex items-start justify-between gap-2">
                   <div><div className="font-medium">{action.label}</div><div className="mt-0.5 break-words leading-relaxed">{action.instructions}</div></div>
                   <button
                     type="button"
                     className="inline-flex shrink-0 items-center gap-1 rounded border border-border px-2 py-1 text-[10px] hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                     aria-label={t("v3.blockers.copyAction")}
                     onClick={() => void navigator.clipboard.writeText(action.copy_text).then(() => {
                       setCopiedAction(action.action_id);
                       window.setTimeout(() => setCopiedAction(undefined), 1500);
                     })}
                   >
                     {copiedAction === action.action_id ? <Check className="h-3 w-3" /> : <Clipboard className="h-3 w-3" />}
                     {t(copiedAction === action.action_id ? "v3.blockers.copied" : "v3.blockers.copy")}
                   </button>
                 </div>
                 <div className="mt-1 text-[10px] text-muted-foreground">{t("v3.blockers.verify", { value: action.verification })}</div>
               </div>)}
             </div>

            {blocker.criterion_id && (
              <div className="mt-2 flex items-center gap-1 text-[10px] text-muted-foreground">
                <CircleHelp className="h-3 w-3" />{t("v3.blockers.criterion")}<span className="font-mono">{blocker.criterion_id}</span>
              </div>
            )}
            {blocker.node_id && (
              <div className="mt-1 flex items-center gap-1 text-[10px] text-muted-foreground">
                <ExternalLink className="h-3 w-3" />{t("v3.blockers.node")}<span className="font-mono">{blocker.node_id}</span>
              </div>
            )}
            <div className="mt-2 text-[10px] text-muted-foreground">{t("v3.blockers.existingEvidence", { count: blocker.evidence_refs.length })}</div>
            <EvidenceLink evidenceRefs={blocker.evidence_refs} className="mt-1" />
          </article>
        ))}
      </div>}
    </section>
  );
}
