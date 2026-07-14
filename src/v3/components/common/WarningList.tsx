import { useEffect, useId, useRef, useState } from "react";
import { AlertTriangle, Info, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import type { Warning } from "@/v3/contracts/generated/project-overview-view";
import { DiagnosticDetails } from "@/v3/components/common/DiagnosticDetails";

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
  const [isOpen, setIsOpen] = useState(false);
  const { t } = useTranslation();
  const panelId = useId();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const reduceMotion = useReducedMotion();

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setIsOpen(false);
        buttonRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  if (!warnings || warnings.length === 0) return null;
  
  // 按照 code 和 message 去重
  const uniqueWarnings = Array.from(new Map(warnings.map(w => [w.code + w.message_key, w])).values());
  const sorted = [...uniqueWarnings].sort((a, b) => {
    const order = { error: 0, warning: 1, info: 2 };
    return order[a.severity] - order[b.severity];
  });

  const topSeverity = sorted[0].severity;
  const topConfig = severityConfig[topSeverity];
  const Icon = topConfig.icon;
  const toneClassName = {
    info: "border-blue-400/60 bg-blue-100 text-blue-700 dark:border-blue-400/50 dark:bg-blue-500/15 dark:text-blue-300",
    warning: "border-amber-400/70 bg-amber-100 text-amber-700 dark:border-amber-400/60 dark:bg-amber-500/15 dark:text-amber-300",
    error: "border-red-400/60 bg-red-100 text-red-700 dark:border-red-400/50 dark:bg-red-500/15 dark:text-red-300",
  }[topSeverity];
  const groupLabel = t("v3.diagnostics.warning.group", { count: sorted.length });

  return (
    <div
      className={cn("relative inline-flex z-50", className)}
      onMouseEnter={() => setIsOpen(true)}
      onMouseLeave={(event) => {
        if (!event.currentTarget.contains(document.activeElement)) setIsOpen(false);
      }}
      onFocus={() => setIsOpen(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setIsOpen(false);
      }}
    >
      <button
        ref={buttonRef}
        type="button"
        aria-expanded={isOpen}
        aria-controls={panelId}
        aria-haspopup="dialog"
        className={cn(
          "inline-flex h-6 w-6 items-center justify-center rounded-md border shadow-sm transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          toneClassName,
          isOpen ? "scale-105 brightness-95" : "hover:scale-105 hover:brightness-95"
        )}
        onClick={() => setIsOpen((value) => !value)}
        title={groupLabel}
        aria-label={groupLabel}
      >
        <Icon className="h-3.5 w-3.5" aria-hidden="true" />
      </button>

      <AnimatePresence>
        {isOpen && (
          <motion.div
            id={panelId}
            role="dialog"
            aria-label={groupLabel}
            initial={reduceMotion ? false : { opacity: 0, y: 4, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduceMotion ? undefined : { opacity: 0, y: 4, scale: 0.95 }}
            transition={reduceMotion ? { duration: 0 } : { duration: 0.15, ease: "easeOut" }}
            className="absolute left-0 top-full z-50 w-80 origin-top-left pt-2"
          >
            <div className="max-h-64 overflow-y-auto scrollbar-auto-hide space-y-1 rounded-md border border-border/50 bg-background p-1.5 shadow-xl">
              {sorted.map((w) => {
                const config = severityConfig[w.severity];
                const WIcon = config.icon;
                return (
                  <div key={w.code + w.message_key} className={cn("flex items-start gap-2 border px-3 py-2 text-sm rounded-sm", config.className)}>
                    <WIcon className="h-4 w-4 shrink-0 mt-0.5" aria-hidden="true" />
                    <DiagnosticDetails diagnostic={w} kind="warning" />
                  </div>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
