import { useState } from "react";
import { AlertCircle, RefreshCw, Layers, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import type { StructuredError } from "@/v3/contracts/generated/project-overview-view";

interface ErrorListProps {
  errors: StructuredError[];
  className?: string;
  onRetry?: () => void;
}

export function ErrorList({ errors, className, onRetry }: ErrorListProps) {
  const [isHovered, setIsHovered] = useState(false);

  if (!errors || errors.length === 0) return null;

  // 按照 code 和 message 去重
  const uniqueErrors = Array.from(new Map(errors.map(err => [err.code + err.message_key, err])).values());

  if (uniqueErrors.length === 1) {
    const err = uniqueErrors[0];
    return (
      <div className={cn("flex items-start gap-2 border border-red-500/20 bg-red-500/5 px-3 py-2 text-sm text-red-700 dark:text-red-400 rounded-md", className)}>
        <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <span className="font-medium">{err.message_key}</span>
          <span className="ml-2 text-xs opacity-70 font-mono">{err.code}</span>
          <span className="ml-2 text-xs opacity-60">[{err.category}]</span>
        </div>
        {err.recoverable && onRetry && (
          <button type="button" onClick={onRetry} className="shrink-0 p-1 hover:bg-red-500/10 transition-colors rounded-sm" title="重试">
            <RefreshCw className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
    );
  }

  const baseColors = "border-red-500/20 bg-red-500/5 text-red-700 dark:text-red-400";

  return (
    <div 
      className={cn("relative z-30", className)}
      onMouseEnter={() => setIsHovered(true)} 
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* 顶部堆叠卡片 */}
      <div className={cn("relative flex cursor-pointer items-center justify-between border px-3 py-2 text-sm shadow-sm transition-all rounded-md", baseColors)}>
        {/* 虚拟堆叠阴影 */}
        <div className={cn("absolute -bottom-1 left-1.5 right-1.5 h-full border opacity-50 z-[-1] rounded-b-md", baseColors)} />
        <div className={cn("absolute -bottom-2 left-3 right-3 h-full border opacity-25 z-[-2] rounded-b-md", baseColors)} />
        
        <div className="flex items-center gap-2 relative z-10">
          <Layers className="h-4 w-4 shrink-0" />
          <span className="font-medium">{uniqueErrors.length} 个错误信息（悬浮展开）</span>
        </div>
        <ChevronDown className={cn("h-4 w-4 transition-transform", isHovered && "rotate-180")} />
      </div>

      {/* 悬浮展开列表 */}
      {isHovered && (
        <div className="absolute top-full left-0 right-0 mt-3 space-y-1 bg-background shadow-xl rounded-md border border-border/50 p-1.5 animate-in fade-in slide-in-from-top-2">
          {uniqueErrors.map((err) => (
            <div key={err.code + err.message_key} className={cn("flex items-start gap-2 border px-3 py-2 text-sm rounded-sm", baseColors)}>
              <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
              <div className="min-w-0 flex-1">
                <span className="font-medium">{err.message_key}</span>
                <span className="ml-2 text-xs opacity-70 font-mono">{err.code}</span>
                <span className="ml-2 text-xs opacity-60">[{err.category}]</span>
              </div>
              {err.recoverable && onRetry && (
                <button type="button" onClick={onRetry} className="shrink-0 p-1 hover:bg-red-500/10 transition-colors rounded-sm" title="重试">
                  <RefreshCw className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
