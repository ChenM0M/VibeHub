import { useState } from "react";
import { Settings, FolderGit2, Code2, Bot, Rocket, X, Link } from "lucide-react";
import { cn } from "@/lib/utils";

interface ProjectSetupModalProps {
  mode: "setup" | "settings";
  onClose: () => void;
  onInitialize: () => void;
  initialLanguage?: string;
}

const PROJECT_LANGUAGES = [
  { id: "zh-CN", name: "简体中文", icon: "简" },
  { id: "zh-TW", name: "繁体中文", icon: "繁" },
  { id: "en-US", name: "English", icon: "EN" },
];

const AI_TOOLS = [
  { id: "claude_code", name: "Claude Code", folder: ".claude", desc: "Anthropic's official CLI agent" },
  { id: "opencode", name: "OpenCode", folder: ".opencode", desc: "Open source multi-agent framework" },
  { id: "cursor", name: "Cursor", folder: ".cursor", desc: "AI-first code editor integration" },
  { id: "codex", name: "Codex", folder: ".codex", desc: "VibeHub's specialized agent" },
];

export function ProjectSetupModal({ mode, onClose, onInitialize, initialLanguage }: ProjectSetupModalProps) {
  const [selectedLang, setSelectedLang] = useState<string | null>(initialLanguage ?? null);
  const [gitUrl, setGitUrl] = useState("");
  const [selectedTools, setSelectedTools] = useState<Set<string>>(new Set());

  const toggleTool = (id: string) => {
    setSelectedTools((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const isSetup = mode === "setup";
  const canSubmit = selectedLang !== null && selectedTools.size > 0;

  return (
    <div className={cn("z-[100] flex items-center justify-center", isSetup ? "absolute inset-0" : "fixed inset-0")}>
      {/* 遮罩层 */}
      <div 
        className={cn("absolute inset-0 bg-background/80 backdrop-blur-sm", isSetup ? "" : "cursor-pointer")} 
        onClick={() => !isSetup && onClose()} 
      />
      
      {/* 模态框 */}
      <div className="relative z-10 w-[560px] max-w-[95vw] max-h-[95%] flex flex-col rounded-xl border border-border bg-card shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        
        {/* 头部 */}
        <div className="flex items-center justify-between border-b border-border/50 bg-muted/20 px-6 py-4">
          <div className="flex items-center gap-2">
            {isSetup ? <Rocket className="h-5 w-5 text-blue-500" /> : <Settings className="h-5 w-5 text-muted-foreground" />}
            <h2 className="text-lg font-semibold tracking-tight">
              {isSetup ? "初始化项目工作区" : "项目设置"}
            </h2>
          </div>
          {!isSetup && (
            <button onClick={onClose} className="rounded p-1 hover:bg-muted text-muted-foreground hover:text-foreground transition-colors">
              <X className="h-4 w-4" />
            </button>
          )}
        </div>

        {/* 内容区 */}
        <div className="flex-1 overflow-y-auto scrollbar-auto-hide p-6 space-y-8">
          
          {/* 1. 主语言选择 */}
          <section className="space-y-3">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <Code2 className="h-4 w-4 text-muted-foreground" />
              1. 选择项目默认语言 <span className="text-red-500">*</span>
            </h3>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              {PROJECT_LANGUAGES.map((lang) => (
                <button
                  key={lang.id}
                  onClick={() => setSelectedLang(lang.id)}
                  className={cn(
                    "flex flex-col items-center justify-center gap-2 rounded-lg border p-3 transition-all",
                    selectedLang === lang.id
                      ? "border-blue-500 bg-blue-500/10 text-blue-600 dark:text-blue-400 ring-1 ring-blue-500"
                      : "border-border/60 hover:bg-muted/50 text-muted-foreground hover:text-foreground"
                  )}
                >
                  <div className="flex h-8 w-8 items-center justify-center rounded bg-background font-mono text-xs font-bold border border-border/50 shadow-sm">
                    {lang.icon}
                  </div>
                  <span className="text-[11px] font-medium">{lang.name}</span>
                </button>
              ))}
            </div>
          </section>

          {/* 2. 远端 Git 配置 */}
          <section className="space-y-3">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <FolderGit2 className="h-4 w-4 text-muted-foreground" />
              2. 远端仓库链接 (可选)
            </h3>
            <div className="relative">
              <Link className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <input 
                type="text" 
                placeholder="https://github.com/user/repo.git"
                value={gitUrl}
                onChange={(e) => setGitUrl(e.target.value)}
                className="w-full rounded-md border border-border/60 bg-background pl-9 pr-4 py-2 text-sm placeholder:text-muted-foreground/50 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 transition-shadow"
              />
            </div>
            <p className="text-[11px] text-muted-foreground">用于生成链接、同步 Issue 及追踪远端进度。</p>
          </section>

          {/* 3. AI 工具集成 */}
          <section className="space-y-3">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <Bot className="h-4 w-4 text-muted-foreground" />
              3. 选择 AI 代理环境 <span className="text-red-500">*</span>
            </h3>
            <div className="space-y-2">
              {AI_TOOLS.map((tool) => (
                <label key={tool.id} className={cn(
                  "flex items-start gap-3 rounded-lg border p-3 cursor-pointer transition-colors",
                  selectedTools.has(tool.id) ? "border-foreground/30 bg-muted/20" : "border-border/40 hover:bg-muted/10"
                )}>
                  <div className="mt-0.5 flex items-center">
                    <input 
                      type="checkbox" 
                      checked={selectedTools.has(tool.id)}
                      onChange={() => toggleTool(tool.id)}
                      className="h-4 w-4 rounded border-border text-blue-600 focus:ring-blue-500"
                    />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{tool.name}</span>
                      <span className="text-[10px] font-mono text-muted-foreground bg-muted px-1 rounded">{tool.folder}</span>
                    </div>
                    <p className="text-xs text-muted-foreground mt-0.5">{tool.desc}</p>
                  </div>
                </label>
              ))}
            </div>
            <p className="text-[11px] text-muted-foreground mt-1">
              勾选后将自动初始化相应的隐藏文件夹并写入 VibeHub 的上下文协议指针。
            </p>
          </section>

        </div>

        {/* 底部操作区 */}
        <div className="border-t border-border/50 bg-muted/10 px-6 py-4 flex justify-end gap-3">
          {!isSetup && (
            <button 
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium rounded-md hover:bg-muted transition-colors"
            >
              取消
            </button>
          )}
          <button 
            onClick={onInitialize}
            disabled={!canSubmit}
            className={cn(
              "px-5 py-2 text-sm font-medium rounded-md flex items-center gap-2 transition-all",
              canSubmit 
                ? "bg-foreground text-background hover:bg-foreground/90 shadow-sm" 
                : "bg-muted text-muted-foreground cursor-not-allowed opacity-70"
            )}
          >
            {isSetup ? (
              <>
                <Rocket className="h-4 w-4" />
                安装和初始化
              </>
            ) : (
              "保存设置"
            )}
          </button>
        </div>

      </div>
    </div>
  );
}
