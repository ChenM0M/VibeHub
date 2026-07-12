import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { NativePath } from "@/v3/contracts/generated/project-overview-view";

const platformLabel: Record<string, string> = {
  macos: "macOS",
  windows: "Win",
  linux: "Linux",
  unknown: "?",
};

interface NativePathDisplayProps {
  path: NativePath;
  className?: string;
  showPlatform?: boolean;
}

export function NativePathDisplay({ path, className, showPlatform = false }: NativePathDisplayProps) {
  const display = path.display || path.native;
  const isLong = display.length > 60;

  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className={cn("font-mono text-xs inline-flex items-center gap-1.5", className)}>
            {showPlatform && (
              <span className="shrink-0 border border-border/70 px-1 text-[10px] text-muted-foreground">
                {platformLabel[path.platform] ?? path.platform}
              </span>
            )}
            <span className={cn("truncate", isLong && "max-w-[400px]")}>{display}</span>
            {path.symlink_target && (
              <span className="shrink-0 text-muted-foreground/60">→ {path.symlink_target}</span>
            )}
          </span>
        </TooltipTrigger>
        {isLong && (
          <TooltipContent side="bottom" className="max-w-[600px]">
            <span className="font-mono text-xs break-all">{path.native}</span>
          </TooltipContent>
        )}
      </Tooltip>
    </TooltipProvider>
  );
}
