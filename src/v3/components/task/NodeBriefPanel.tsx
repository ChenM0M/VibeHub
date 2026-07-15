import { StateBadge } from "@/v3/components/common/StateBadge";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { NativePathDisplay } from "@/v3/components/common/NativePathDisplay";
import { AcceptanceProgress } from "@/v3/components/task/AcceptanceProgress";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import { Badge } from "@/components/ui/badge";
import type { NodeBrief } from "@/v3/contracts/generated/node-brief";

const stateLabel: Record<string, string> = {
  planned: "待执行", ready: "就绪", active: "进行中", blocked: "阻塞", review: "审查中", completed: "已完成", cancelled: "已取消", superseded: "已替代",
};

function SubSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border border-border/70 p-4">
      <div className="mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wide">{title}</div>
      {children}
    </div>
  );
}
function TextList({ items, muted }: { items: string[]; muted?: boolean }) {
  if (items.length === 0) return <p className="text-sm text-muted-foreground italic">无</p>;
  return <ul className={"list-disc list-inside text-sm space-y-0.5 " + (muted ? "text-muted-foreground" : "")}>{items.map((s, i) => <li key={i}>{s}</li>)}</ul>;
}

interface NodeBriefPanelProps { data: NodeBrief; }

export function NodeBriefPanel({ data }: NodeBriefPanelProps) {
  const budgetUsed = data.budget.max_tokens > 0 ? (data.budget.estimated_tokens / data.budget.max_tokens) * 100 : 0;
  return (
    <div className="space-y-4 p-6">
      {/* 标题区 */}
      <div className="flex items-start justify-between gap-4 pb-3 border-b border-border/60">
        <div className="min-w-0 flex-1">
          <h3 className="text-lg font-semibold">{data.goal}</h3>
          <div className="mt-1.5 flex items-center gap-2">
            <Badge variant="outline">{stateLabel[data.state] ?? data.state}</Badge>
            <span className="font-mono text-[11px] text-muted-foreground">{data.node_id}</span>
          </div>
        </div>
        <StateBadge freshness={data.freshness} completeness={data.completeness} />
      </div>

      {data.next_intent && (
        <div className="border border-blue-500/20 bg-blue-500/5 px-3 py-2 text-sm">
          <span className="text-muted-foreground">下一步意图：</span>{data.next_intent}
        </div>
      )}

      <BlockerDetailsPanel blockers={data.blocker_details} />

      <div className="grid grid-cols-2 gap-4">
        <SubSection title="范围"><TextList items={data.scope} /></SubSection>
        <SubSection title="不在范围内"><TextList items={data.non_scope} muted /></SubSection>
        <SubSection title="依赖"><TextList items={data.dependencies} /></SubSection>
        <SubSection title="已接受决策"><TextList items={data.accepted_decisions} /></SubSection>
      </div>

      {data.research_summary.length > 0 && <SubSection title="研究摘要"><TextList items={data.research_summary} /></SubSection>}

      <SubSection title="关联文件">
        {data.files.length === 0 ? <p className="text-sm text-muted-foreground italic">无关联文件</p> : (
          <div className="space-y-1">{data.files.map((file, i) => <div key={i}><NativePathDisplay path={file} showPlatform /></div>)}</div>
        )}
      </SubSection>

      <SubSection title="验证命令">
        {data.validation_commands.length === 0 ? <p className="text-sm text-muted-foreground italic">无验证命令</p> : (
          <div className="space-y-1">{data.validation_commands.map((cmd, i) => <pre key={i} className="bg-muted p-2 text-xs font-mono whitespace-pre-wrap break-all">{cmd}</pre>)}</div>
        )}
      </SubSection>

      <div className="grid grid-cols-2 gap-4">
        <SubSection title="预算">
          <div className="space-y-1.5 text-sm">
            <div className="flex justify-between"><span className="text-muted-foreground">上限</span><span>{data.budget.max_tokens.toLocaleString()}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">预估</span><span>{data.budget.estimated_tokens.toLocaleString()}（{budgetUsed.toFixed(0)}%）</span></div>
            <div className="h-1.5 bg-muted"><div className="h-full bg-amber-500" style={{ width: `${Math.min(budgetUsed, 100)}%` }} /></div>
            {data.budget.truncated_sections.length > 0 && <div className="text-orange-600 text-xs">已截断：{data.budget.truncated_sections.join("、")}</div>}
          </div>
        </SubSection>
        {Object.keys(data.source_versions).length > 0 && (
          <SubSection title="源版本">
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
