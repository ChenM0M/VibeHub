import { useEffect, useState } from "react";
import { Settings, FolderGit2, Code2, Bot, Rocket, X, Link, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { V3AgentSpecInspection, V3AgentSpecTarget, V3OutputLanguage, V3ProjectSettingsUpdateRequest } from "@/v3/contracts";

interface ProjectSetupModalProps {
  mode: "setup" | "settings";
  onClose: () => void;
  onSubmit: (request: V3ProjectSettingsUpdateRequest) => Promise<boolean>;
  onForceSync?: () => Promise<boolean>;
  initialLanguage?: V3OutputLanguage;
  initialTools?: V3AgentSpecTarget[];
  initialGitUrl?: string;
  expectedRevision?: number;
  agentSpecs?: V3AgentSpecInspection | null;
  pending?: boolean;
  error?: string | null;
}

const PROJECT_LANGUAGES: { id: V3OutputLanguage; name: string; icon: string }[] = [
  { id: "zh-CN", name: "简体中文", icon: "简" },
  { id: "zh-TW", name: "繁体中文", icon: "繁" },
  { id: "en-US", name: "English", icon: "EN" },
];

const AI_TOOLS: { id: V3AgentSpecTarget | "cursor"; name: string; folder: string; desc: string; disabled?: boolean }[] = [
  { id: "claude_code", name: "Claude Code", folder: "CLAUDE.md", desc: "Claude Code 项目指令" },
  { id: "opencode", name: "OpenCode", folder: "AGENTS.md", desc: "共享 Agent 项目指令" },
  { id: "cursor", name: "Cursor", folder: ".cursor", desc: "当前 V3 agent spec 不支持", disabled: true },
  { id: "codex", name: "Codex", folder: "AGENTS.md", desc: "共享 Agent 项目指令" },
];

const statusLabel = { missing: "缺失", in_sync: "已同步", outdated: "已过期", modified_outside: "外部修改", unsupported: "不支持" };
const statusTone = { missing: "text-amber-600", in_sync: "text-emerald-600", outdated: "text-amber-600", modified_outside: "text-orange-600", unsupported: "text-muted-foreground" };

const EMPTY_TARGETS: V3AgentSpecTarget[] = [];

export function ProjectSetupModal({ mode, onClose, onSubmit, onForceSync, initialLanguage = "zh-CN", initialTools = EMPTY_TARGETS, initialGitUrl = "", expectedRevision = 0, agentSpecs, pending = false, error }: ProjectSetupModalProps) {
  const [selectedLang, setSelectedLang] = useState<V3OutputLanguage>(initialLanguage);
  const [gitUrl, setGitUrl] = useState(initialGitUrl);
  const [selectedTools, setSelectedTools] = useState<Set<V3AgentSpecTarget>>(new Set(initialTools));
  const [confirmForce, setConfirmForce] = useState(false);

  useEffect(() => {
    setSelectedLang(initialLanguage);
    setGitUrl(initialGitUrl);
    setSelectedTools(new Set(initialTools));
  }, [initialLanguage, initialGitUrl, initialTools]);

  const toggleTool = (id: V3AgentSpecTarget) => setSelectedTools((previous) => {
    const next = new Set(previous);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });
  const isSetup = mode === "setup";
  const canSubmit = selectedTools.size > 0 && !pending;
  const modifiedOutside = agentSpecs?.artifacts.some((artifact) => artifact.status === "modified_outside") ?? false;

  const submit = async () => {
    const succeeded = await onSubmit({
      expected_revision: expectedRevision,
      output_language: selectedLang,
      agent_spec_targets: Array.from(selectedTools),
      repository_remote_url: gitUrl.trim() || null,
    });
    if (succeeded) onClose();
  };

  return (
    <div className={cn("z-[100] flex items-center justify-center", isSetup ? "absolute inset-0" : "fixed inset-0")}>
      <div className={cn("absolute inset-0 bg-background/80 backdrop-blur-sm", isSetup ? "" : "cursor-pointer")} onClick={() => !isSetup && !pending && onClose()} />
      <div className="relative z-10 flex max-h-[95%] w-[560px] max-w-[95vw] flex-col overflow-hidden rounded-xl border border-border bg-card shadow-2xl animate-in fade-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between border-b border-border/50 bg-muted/20 px-6 py-4">
          <div className="flex items-center gap-2">{isSetup ? <Rocket className="h-5 w-5 text-blue-500" /> : <Settings className="h-5 w-5 text-muted-foreground" />}<h2 className="text-lg font-semibold tracking-tight">{isSetup ? "初始化项目配置" : "项目设置"}</h2></div>
          {!isSetup && <button onClick={onClose} disabled={pending} className="rounded p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"><X className="h-4 w-4" /></button>}
        </div>
        <div className="flex-1 space-y-8 overflow-y-auto p-6 scrollbar-auto-hide">
          <section className="space-y-3">
            <h3 className="flex items-center gap-2 text-sm font-semibold"><Code2 className="h-4 w-4 text-muted-foreground" />1. 自然语言输出语言 <span className="text-red-500">*</span></h3>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">{PROJECT_LANGUAGES.map((language) => <button key={language.id} type="button" onClick={() => setSelectedLang(language.id)} className={cn("flex flex-col items-center justify-center gap-2 rounded-lg border p-3 transition-all", selectedLang === language.id ? "border-blue-500 bg-blue-500/10 text-blue-600 ring-1 ring-blue-500 dark:text-blue-400" : "border-border/60 text-muted-foreground hover:bg-muted/50 hover:text-foreground")}><div className="flex h-8 w-8 items-center justify-center rounded border border-border/50 bg-background font-mono text-xs font-bold shadow-sm">{language.icon}</div><span className="text-[11px] font-medium">{language.name}</span></button>)}</div>
          </section>
          <section className="space-y-3">
            <h3 className="flex items-center gap-2 text-sm font-semibold"><FolderGit2 className="h-4 w-4 text-muted-foreground" />2. 远端仓库链接（可选）</h3>
            <div className="relative"><Link className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" /><input type="text" placeholder="https://github.com/user/repo.git" value={gitUrl} onChange={(event) => setGitUrl(event.target.value)} className="w-full rounded-md border border-border/60 bg-background py-2 pl-9 pr-4 text-sm transition-shadow placeholder:text-muted-foreground/50 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500" /></div>
            <p className="text-[11px] text-muted-foreground">仅保存为项目 metadata；不会修改 Git remote、同步 Issue 或执行网络操作。</p>
          </section>
          <section className="space-y-3">
            <h3 className="flex items-center gap-2 text-sm font-semibold"><Bot className="h-4 w-4 text-muted-foreground" />3. 选择 AI 代理环境 <span className="text-red-500">*</span></h3>
            <div className="space-y-2">{AI_TOOLS.map((tool) => {
              const selected = tool.id !== "cursor" && selectedTools.has(tool.id);
              return <label key={tool.id} className={cn("flex items-start gap-3 rounded-lg border p-3 transition-colors", tool.disabled ? "cursor-not-allowed border-border/30 opacity-55" : "cursor-pointer", selected ? "border-foreground/30 bg-muted/20" : "border-border/40 hover:bg-muted/10")}><div className="mt-0.5 flex items-center"><input type="checkbox" disabled={tool.disabled} checked={selected} onChange={() => tool.id !== "cursor" && toggleTool(tool.id)} className="h-4 w-4 rounded border-border text-blue-600 focus:ring-blue-500" /></div><div className="min-w-0 flex-1"><div className="flex items-center gap-2"><span className="text-sm font-medium">{tool.name}</span><span className="rounded bg-muted px-1 font-mono text-[10px] text-muted-foreground">{tool.folder}</span>{tool.disabled && <span className="text-[10px] text-muted-foreground">unsupported</span>}</div><p className="mt-0.5 text-xs text-muted-foreground">{tool.desc}</p></div></label>;
            })}</div>
          </section>
          {agentSpecs && <section className="space-y-2"><h3 className="text-sm font-semibold">Agent spec 状态</h3>{agentSpecs.artifacts.map((artifact) => <div key={artifact.path} className="rounded-md border border-border/50 px-3 py-2 text-xs"><div className="flex items-center justify-between gap-3"><span className="font-mono font-medium">{artifact.path}</span><span className={statusTone[artifact.status]}>{statusLabel[artifact.status]}</span></div><div className="mt-1 text-muted-foreground">{artifact.reason}</div></div>)}{modifiedOutside && onForceSync && <div className="rounded-md border border-orange-500/30 bg-orange-500/5 p-3 text-xs"><div className="flex items-start gap-2"><AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-orange-600" /><div><div className="font-medium">检测到 managed region 被外部修改</div><p className="mt-1 text-muted-foreground">普通更新会跳过这些文件。强制同步只替换 VibeHub managed region，保留区域外内容。</p></div></div>{!confirmForce ? <button type="button" className="mt-2 text-orange-700 underline dark:text-orange-400" onClick={() => setConfirmForce(true)}>强制同步…</button> : <div className="mt-2 flex items-center gap-2"><button type="button" className="rounded bg-orange-600 px-2 py-1 text-white" onClick={async () => { if (await onForceSync()) setConfirmForce(false); }}>确认仅替换 managed region</button><button type="button" className="underline" onClick={() => setConfirmForce(false)}>取消</button></div>}</div>}</section>}
          {error && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">{error}</div>}
        </div>
        <div className="flex justify-end gap-3 border-t border-border/50 bg-muted/10 px-6 py-4">{!isSetup && <button onClick={onClose} disabled={pending} className="rounded-md px-4 py-2 text-sm font-medium transition-colors hover:bg-muted">取消</button>}<button onClick={() => void submit()} disabled={!canSubmit} className={cn("flex items-center gap-2 rounded-md px-5 py-2 text-sm font-medium transition-all", canSubmit ? "bg-foreground text-background shadow-sm hover:bg-foreground/90" : "cursor-not-allowed bg-muted text-muted-foreground opacity-70")}>{pending ? "更新中…" : "更新项目配置"}</button></div>
      </div>
    </div>
  );
}
