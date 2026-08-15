import { useEffect, useRef, useState } from "react";
import { Settings, FolderGit2, Code2, Bot, Rocket, X, Link, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { V3AgentSpecInspection, V3AgentSpecTarget, V3OutputLanguage, V3ProjectSettingsUpdateRequest } from "@/v3/contracts";
import { projectSetupSignature, shouldAdoptProjectSetupValues } from "./projectSetupFormSync";
import { useTranslation } from "react-i18next";

interface ProjectSetupModalProps {
  mode: "setup" | "settings";
  onClose: () => void;
  onSubmit: (request: V3ProjectSettingsUpdateRequest) => Promise<boolean>;
  onSync?: () => Promise<boolean>;
  onForceSync?: () => Promise<boolean>;
  initialLanguage?: V3OutputLanguage;
  initialTools?: V3AgentSpecTarget[];
  initialGitUrl?: string;
  expectedRevision?: number;
  agentSpecs?: V3AgentSpecInspection | null;
  pending?: boolean;
  error?: string | null;
}

const PROJECT_LANGUAGES: { id: V3OutputLanguage; nameKey: string; icon: string }[] = [
  { id: "zh-CN", nameKey: "v3.setup.language.zh-CN", icon: "CN" },
  { id: "zh-TW", nameKey: "v3.setup.language.zh-TW", icon: "TW" },
  { id: "en-US", nameKey: "v3.setup.language.en-US", icon: "EN" },
];

const AI_TOOLS: { id: V3AgentSpecTarget | "cursor"; name: string; folder: string; descKey: string; disabled?: boolean }[] = [
  { id: "claude_code", name: "Claude Code", folder: "CLAUDE.md", descKey: "v3.setup.tool.claude_code" },
  { id: "opencode", name: "OpenCode", folder: "AGENTS.md", descKey: "v3.setup.tool.opencode" },
  { id: "cursor", name: "Cursor", folder: ".cursor", descKey: "v3.setup.tool.cursor", disabled: true },
  { id: "codex", name: "Codex", folder: "AGENTS.md", descKey: "v3.setup.tool.codex" },
];

const statusTone = { missing: "text-amber-600", in_sync: "text-emerald-600", outdated: "text-amber-600", modified_outside: "text-orange-600", legacy_migratable: "text-orange-600", unsupported: "text-red-600" };
const hostStatusTone: Record<string, string> = { missing: "text-amber-600", in_sync: "text-emerald-600", mismatched: "text-red-600", invalid: "text-red-600", unsupported: "text-red-600", ambiguous: "text-orange-600" };

const EMPTY_TARGETS: V3AgentSpecTarget[] = [];

export function ProjectSetupModal({ mode, onClose, onSubmit, onSync, onForceSync, initialLanguage = "zh-CN", initialTools = EMPTY_TARGETS, initialGitUrl = "", expectedRevision = 0, agentSpecs, pending = false, error }: ProjectSetupModalProps) {
  const { t } = useTranslation();
  const [selectedLang, setSelectedLang] = useState<V3OutputLanguage>(initialLanguage);
  const [gitUrl, setGitUrl] = useState(initialGitUrl);
  const [selectedTools, setSelectedTools] = useState<Set<V3AgentSpecTarget>>(new Set(initialTools));
  const [confirmForce, setConfirmForce] = useState(false);
  const initialSignature = projectSetupSignature({ language: initialLanguage, gitUrl: initialGitUrl, tools: initialTools, revision: expectedRevision });
  const syncedSignature = useRef(initialSignature);
  const dirty = useRef(false);

  useEffect(() => {
    if (initialSignature === syncedSignature.current) return;
    const adopt = shouldAdoptProjectSetupValues({ nextSignature: initialSignature, syncedSignature: syncedSignature.current, dirty: dirty.current });
    syncedSignature.current = initialSignature;
    if (!adopt) return;
    setSelectedLang(initialLanguage);
    setGitUrl(initialGitUrl);
    setSelectedTools(new Set(initialTools));
  }, [initialSignature, initialLanguage, initialGitUrl, initialTools]);

  const toggleTool = (id: V3AgentSpecTarget) => setSelectedTools((previous) => {
    dirty.current = true;
    const next = new Set(previous);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });
  const isSetup = mode === "setup";
  const canSubmit = selectedTools.size > 0 && !pending;
  const needsOrdinarySync = agentSpecs?.artifacts.some((artifact) => artifact.status === "missing" || artifact.status === "outdated") ?? false;
  const needsConfirmedSync = agentSpecs?.artifacts.some((artifact) => artifact.status === "modified_outside" || artifact.status === "legacy_migratable") ?? false;
  const hasLegacy = agentSpecs?.artifacts.some((artifact) => artifact.status === "legacy_migratable") ?? false;
  const hasUnsupported = agentSpecs?.artifacts.some((artifact) => artifact.status === "unsupported") ?? false;

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
          <div className="flex items-center gap-2">{isSetup ? <Rocket className="h-5 w-5 text-blue-500" /> : <Settings className="h-5 w-5 text-muted-foreground" />}<h2 className="text-lg font-semibold tracking-tight">{t(isSetup ? "v3.setup.initializeTitle" : "v3.setup.settingsTitle")}</h2></div>
          {!isSetup && <button onClick={onClose} disabled={pending} className="rounded p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"><X className="h-4 w-4" /></button>}
        </div>
        <div className="flex-1 space-y-8 overflow-y-auto p-6 scrollbar-auto-hide">
          <section className="space-y-3">
            <h3 className="flex items-center gap-2 text-sm font-semibold"><Code2 className="h-4 w-4 text-muted-foreground" />{t("v3.setup.outputLanguage")} <span className="text-red-500">*</span></h3>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">{PROJECT_LANGUAGES.map((language) => <button key={language.id} type="button" onClick={() => { dirty.current = true; setSelectedLang(language.id); }} className={cn("flex flex-col items-center justify-center gap-2 rounded-lg border p-3 transition-all", selectedLang === language.id ? "border-blue-500 bg-blue-500/10 text-blue-600 ring-1 ring-blue-500 dark:text-blue-400" : "border-border/60 text-muted-foreground hover:bg-muted/50 hover:text-foreground")}><div className="flex h-8 w-8 items-center justify-center rounded border border-border/50 bg-background font-mono text-xs font-bold shadow-sm">{language.icon}</div><span className="text-[11px] font-medium">{t(language.nameKey)}</span></button>)}</div>
          </section>
          <section className="space-y-3">
            <h3 className="flex items-center gap-2 text-sm font-semibold"><FolderGit2 className="h-4 w-4 text-muted-foreground" />{t("v3.setup.remoteRepository")}</h3>
            <div className="relative"><Link className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" /><input type="text" placeholder="https://github.com/user/repo.git" value={gitUrl} onChange={(event) => { dirty.current = true; setGitUrl(event.target.value); }} className="w-full rounded-md border border-border/60 bg-background py-2 pl-9 pr-4 text-sm transition-shadow placeholder:text-muted-foreground/50 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500" /></div>
            <p className="text-[11px] text-muted-foreground">{t("v3.setup.remoteHint")}</p>
          </section>
          <section className="space-y-3">
            <h3 className="flex items-center gap-2 text-sm font-semibold"><Bot className="h-4 w-4 text-muted-foreground" />{t("v3.setup.selectAgents")} <span className="text-red-500">*</span></h3>
            <div className="space-y-2">{AI_TOOLS.map((tool) => {
              const selected = tool.id !== "cursor" && selectedTools.has(tool.id);
              return <label key={tool.id} className={cn("flex items-start gap-3 rounded-lg border p-3 transition-colors", tool.disabled ? "cursor-not-allowed border-border/30 opacity-55" : "cursor-pointer", selected ? "border-foreground/30 bg-muted/20" : "border-border/40 hover:bg-muted/10")}><div className="mt-0.5 flex items-center"><input type="checkbox" disabled={tool.disabled} checked={selected} onChange={() => tool.id !== "cursor" && toggleTool(tool.id)} className="h-4 w-4 rounded border-border text-blue-600 focus:ring-blue-500" /></div><div className="min-w-0 flex-1"><div className="flex items-center gap-2"><span className="text-sm font-medium">{tool.name}</span><span className="rounded bg-muted px-1 font-mono text-[10px] text-muted-foreground">{tool.folder}</span>{tool.disabled && <span className="text-[10px] text-muted-foreground">{t("v3.setup.unsupported")}</span>}</div><p className="mt-0.5 text-xs text-muted-foreground">{t(tool.descKey)}</p></div></label>;
            })}</div>
          </section>
          {agentSpecs && <section className="space-y-2"><h3 className="text-sm font-semibold">{t("v3.setup.scopeStatus")}</h3><div className="rounded-md border border-border/50 px-3 py-2 text-xs"><div className="grid gap-1 text-muted-foreground"><div><span className="font-medium text-foreground">{t("v3.setup.controlRoot")}：</span><span className="break-all font-mono">{agentSpecs.scope.control_root}</span></div><div><span className="font-medium text-foreground">{t("v3.setup.executionRoot")}：</span><span className="break-all font-mono">{agentSpecs.scope.execution_root}</span></div><div><span className="font-medium text-foreground">{t("v3.setup.hostConfigRoot")}：</span><span className="break-all font-mono">{agentSpecs.scope.host_config_root}</span></div></div>{agentSpecs.scope.warnings.map((warning) => <div key={warning} className="mt-2 text-orange-600">{warning}</div>)}</div>
            <h3 className="pt-2 text-sm font-semibold">{t("v3.setup.effectiveDeclarations")}</h3>{agentSpecs.effective_declarations.map((declaration) => <div key={`${declaration.path}-${declaration.consumers.join("-")}`} className="flex items-center justify-between gap-3 rounded-md border border-border/50 px-3 py-2 text-xs"><span className="break-all font-mono">{declaration.path}</span><span className={declaration.exists ? "text-foreground" : "text-muted-foreground"}>{t(declaration.exists ? "v3.setup.declarationLoaded" : "v3.setup.declarationMissing")} · P{declaration.precedence}</span></div>)}
            <h3 className="pt-2 text-sm font-semibold">{t("v3.setup.mcpHostStatus")}</h3>{agentSpecs.mcp_hosts.map((host) => <div key={`${host.consumer}-${host.path}`} className="rounded-md border border-border/50 px-3 py-2 text-xs"><div className="flex items-center justify-between gap-3"><span className="break-all font-mono">{host.path}</span><span className={hostStatusTone[host.status]}>{t(`v3.setup.hostState.${host.status}`)}</span></div><div className="mt-1 text-muted-foreground">{host.reason}</div>{host.configured_project_root && <div className="mt-1 break-all font-mono text-red-600">{t("v3.setup.configuredRoot")}: {host.configured_project_root}</div>}</div>)}
            <h3 className="pt-2 text-sm font-semibold">{t("v3.setup.specStatus")}</h3>{agentSpecs.artifacts.map((artifact) => <div key={artifact.path} className="rounded-md border border-border/50 px-3 py-2 text-xs"><div className="flex items-center justify-between gap-3"><span className="font-mono font-medium">{artifact.path}</span><span className={statusTone[artifact.status]}>{t(`v3.setup.specState.${artifact.status}`)}</span></div><div className="mt-1 text-muted-foreground">{artifact.reason}</div></div>)}
            {needsOrdinarySync && onSync && <div className="rounded-md border border-amber-500/30 bg-amber-500/5 p-3 text-xs"><p className="text-muted-foreground">{t("v3.setup.syncHint")}</p><button type="button" disabled={pending} className="mt-2 text-amber-700 underline disabled:opacity-50 dark:text-amber-400" onClick={() => void onSync()}>{t("v3.setup.syncNow")}</button></div>}
            {needsConfirmedSync && onForceSync && <div className="rounded-md border border-orange-500/30 bg-orange-500/5 p-3 text-xs"><div className="flex items-start gap-2"><AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-orange-600" /><div><div className="font-medium">{t(hasLegacy ? "v3.setup.legacyTitle" : "v3.setup.modifiedTitle")}</div><p className="mt-1 text-muted-foreground">{t(hasLegacy ? "v3.setup.legacyHint" : "v3.setup.modifiedHint")}</p></div></div>{!confirmForce ? <button type="button" disabled={pending} className="mt-2 text-orange-700 underline disabled:opacity-50 dark:text-orange-400" onClick={() => setConfirmForce(true)}>{t(hasLegacy ? "v3.setup.migrateLegacy" : "v3.setup.forceSync")}</button> : <div className="mt-2 flex items-center gap-2"><button type="button" disabled={pending} className="rounded bg-orange-600 px-2 py-1 text-white disabled:opacity-50" onClick={async () => { if (await onForceSync()) setConfirmForce(false); }}>{t(hasLegacy ? "v3.setup.confirmLegacyMigration" : "v3.setup.confirmForceSync")}</button><button type="button" disabled={pending} className="underline disabled:opacity-50" onClick={() => setConfirmForce(false)}>{t("v3.common.cancel")}</button></div>}</div>}
            {hasUnsupported && <div className="rounded-md border border-red-500/30 bg-red-500/5 p-3 text-xs text-muted-foreground">{t("v3.setup.unsupportedHint")}</div>}
          </section>}
          {error && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">{error}</div>}
        </div>
        <div className="flex justify-end gap-3 border-t border-border/50 bg-muted/10 px-6 py-4">{!isSetup && <button onClick={onClose} disabled={pending} className="rounded-md px-4 py-2 text-sm font-medium transition-colors hover:bg-muted">{t("v3.common.cancel")}</button>}<button onClick={() => void submit()} disabled={!canSubmit} className={cn("flex items-center gap-2 rounded-md px-5 py-2 text-sm font-medium transition-all", canSubmit ? "bg-foreground text-background shadow-sm hover:bg-foreground/90" : "cursor-not-allowed bg-muted text-muted-foreground opacity-70")}>{t(pending ? "v3.setup.updating" : "v3.setup.update")}</button></div>
      </div>
    </div>
  );
}
