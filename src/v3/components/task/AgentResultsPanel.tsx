import { CheckCircle2, Clock3, FileText, SearchCheck, XCircle } from "lucide-react";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { AgentResultsView } from "@/v3/contracts/generated/agent-results-view";

const stateCopy: Record<AgentResultsView["state"], { title: string; detail: string }> = {
  not_executed: { title: "Agent 尚未执行", detail: "当前任务还没有可核验的 Agent 会话。" },
  awaiting_result: { title: "Agent 尚未产出结果", detail: "已经观察到执行会话，但还没有记录最终执行或评估结果。" },
  available: { title: "Agent 结果已就绪", detail: "以下内容来自 V3 结果事件投影。" },
  failed: { title: "Agent 执行包含失败结果", detail: "请查看失败结果及其证据。" },
};

const statusIcon = {
  pending: Clock3,
  running: Clock3,
  succeeded: CheckCircle2,
  failed: XCircle,
};

export function AgentResultsPanel({ data }: { data: AgentResultsView }) {
  const copy = stateCopy[data.state];
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
        return (
          <article key={result.result_id} className="rounded-md border border-border/50 bg-card/40 p-4">
            <div className="flex items-start gap-2">
              <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h4 className="text-sm font-semibold">{result.summary || (result.kind === "evaluation" ? "评估结果" : "执行结果")}</h4>
                  <span className="rounded border border-border/60 px-1.5 py-0.5 text-[10px] text-muted-foreground">{result.kind === "evaluation" ? "评估" : "执行"}</span>
                  <span className="rounded border border-border/60 px-1.5 py-0.5 text-[10px] text-muted-foreground">{runtimeStatus}</span>
                </div>
                <div className="mt-1 text-[11px] text-muted-foreground">{result.request.source === "evaluation_instruction" ? "评估指令" : "用户要求"}：{result.request.instruction}</div>
              </div>
              {result.completed_at && <time className="shrink-0 text-[10px] text-muted-foreground">{new Date(result.completed_at).toLocaleString()}</time>}
            </div>

            {result.body && <p className="mt-3 whitespace-pre-wrap text-sm leading-relaxed text-foreground/85">{result.body}</p>}

            {result.evaluation && (
              <section className="mt-3 rounded border border-border/40 bg-muted/10 p-3">
                <div className="flex items-center gap-2 text-xs font-semibold"><SearchCheck className="h-3.5 w-3.5" />{result.evaluation.target}<span className="ml-auto uppercase text-muted-foreground">{result.evaluation.verdict}</span></div>
                <div className="mt-2 text-[11px] text-muted-foreground">标准：{result.evaluation.rubric.join("；")}</div>
                {result.evaluation.findings.length > 0 && <div className="mt-2 space-y-2">{result.evaluation.findings.map((finding, index) => <div key={`${result.result_id}-${index}`} className="border-l-2 border-border pl-2"><div className="text-xs font-medium">{finding.title} · {finding.severity}</div><p className="text-[11px] text-muted-foreground">{finding.detail}</p><EvidenceLink evidenceRefs={finding.evidence_refs} /></div>)}</div>}
              </section>
            )}

            {result.artifacts.length > 0 && <div className="mt-3 flex flex-wrap gap-2">{result.artifacts.map((artifact, index) => <div key={`${result.result_id}-artifact-${index}`} className="inline-flex items-center gap-1.5 rounded border border-border/50 px-2 py-1 text-[11px]"><FileText className="h-3 w-3" />{artifact.label}{artifact.path && <span className="max-w-48 truncate text-muted-foreground">{artifact.path.display}</span>}</div>)}</div>}
            <div className="mt-3"><EvidenceLink evidenceRefs={result.evidence_refs} /></div>
          </article>
        );
      })}
    </div>
  );
}
