import { useState } from "react";
import { AlertTriangle, Info, XCircle, Layers, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import type { Warning } from "@/v3/contracts/generated/project-overview-view";

const severityConfig = {
  info: { icon: Info, className: "border-blue-500/20 bg-blue-500/5 text-blue-700 dark:text-blue-400" },
  warning: { icon: AlertTriangle, className: "border-amber-500/20 bg-amber-500/5 text-amber-700 dark:text-amber-400" },
  error: { icon: XCircle, className: "border-red-500/20 bg-red-500/5 text-red-700 dark:text-red-400" },
};

interface WarningListProps {
  warnings: Warning[];
  className?: string;
}

export function WarningList({ warnings, className }: WarningListProps) {
  const [isHovered, setIsHovered] = useState(false);
  
  if (!warnings || warnings.length === 0) return null;
  
  // 按照 code 和 message 去重
  const uniqueWarnings = Array.from(new Map(warnings.map(w => [w.code + w.message_key, w])).values());
  const sorted = [...uniqueWarnings].sort((a, b) => {
    const order = { error: 0, warning: 1, info: 2 };
    return order[a.severity] - order[b.severity];
  });

  if (sorted.length === 1) {
    const w = sorted[0];
    const config = severityConfig[w.severity];
    const Icon = config.icon;
    return (
      <div className={cn("flex items-start gap-2 border px-3 py-2 text-sm rounded-md", config.className, className)}>
        <Icon className="h-4 w-4 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <span className="font-medium">{w.message_key}</span>
          <span className="ml-2 text-xs opacity-70 font-mono">{w.code}</span>
        </div>
      </div>
    );
  }

  const topSeverity = sorted[0].severity;
  const topConfig = severityConfig[topSeverity];

  return (
    <div 
      className={cn("relative z-30", className)} 
      onMouseEnter={() => setIsHovered(true)} 
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* 顶部堆叠卡片 */}
      <div className={cn("relative flex cursor-pointer items-center justify-between border px-3 py-2 text-sm shadow-sm transition-all rounded-md", topConfig.className)}>
        {/* 虚拟堆叠阴影 */}
        <div className={cn("absolute -bottom-1 left-1.5 right-1.5 h-full border opacity-50 z-[-1] rounded-b-md", topConfig.className)} />
        <div className={cn("absolute -bottom-2 left-3 right-3 h-full border opacity-25 z-[-2] rounded-b-md", topConfig.className)} />
        
        <div className="flex items-center gap-2 relative z-10">
          <Layers className="h-4 w-4 shrink-0" />
          <span className="font-medium">{sorted.length} 个环境/配置提示信息（悬浮展开）</span>
        </div>
        <ChevronDown className={cn("h-4 w-4 transition-transform", isHovered && "rotate-180")} />
      </div>

      {/* 悬浮展开列表 */}
      {isHovered && (
        <div className="absolute top-full left-0 right-0 mt-3 space-y-1 bg-background shadow-xl rounded-md border border-border/50 p-1.5 animate-in fade-in slide-in-from-top-2">
          {sorted.map((w) => {
            const config = severityConfig[w.severity];
            const Icon = config.icon;
            return (
              <div key={w.code + w.message_key} className={cn("flex items-start gap-2 border px-3 py-2 text-sm rounded-sm", config.className)}>
                <Icon className="h-4 w-4 shrink-0 mt-0.5" />
                <div className="min-w-0">
                  <span className="font-medium">{w.message_key}</span>
                  <span className="ml-2 text-xs opacity-70 font-mono">{w.code}</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
