import { useEffect, useId, useRef, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import type { StructuredError } from "@/v3/contracts/generated/project-overview-view";
import { DiagnosticDetails } from "@/v3/components/common/DiagnosticDetails";

interface ErrorListProps {
  errors: StructuredError[];
  className?: string;
  onRetry?: () => void;
}

export function ErrorList({ errors, className, onRetry }: ErrorListProps) {
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

  if (!errors || errors.length === 0) return null;

  // 按照 code 和 message 去重
  const uniqueErrors = Array.from(new Map(errors.map(err => [err.code + err.message_key, err])).values());
  const baseColors = "border-red-500/20 bg-red-500/5 text-red-700 dark:text-red-400";
  const toneClassName = "border-red-400/60 bg-red-100 text-red-700 dark:border-red-400/50 dark:bg-red-500/15 dark:text-red-300";
  const groupLabel = t("v3.diagnostics.error.group", { count: uniqueErrors.length });

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
        <AlertCircle className="h-3.5 w-3.5" aria-hidden="true" />
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
              {uniqueErrors.map((err) => (
                <div key={err.code + err.message_key} className={cn("flex items-start gap-2 border px-3 py-2 text-sm rounded-sm", baseColors)}>
                  <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" aria-hidden="true" />
                  <DiagnosticDetails diagnostic={err} kind="error" />
                  {err.recoverable && onRetry && (
                    <button type="button" onClick={onRetry} className="shrink-0 p-1 hover:bg-red-500/10 transition-colors rounded-sm" title={t("v3.diagnostics.error.retry")} aria-label={t("v3.diagnostics.error.retry")}>
                      <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" />
                    </button>
                  )}
                </div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
