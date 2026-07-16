import { useTranslation } from "react-i18next";
import type { Diagnostic } from "@/v3/components/common/diagnosticLabels";
import { diagnosticDetailEntries, diagnosticTranslationKey, isKnownDiagnostic } from "@/v3/components/common/diagnosticLabels";

interface DiagnosticDetailsProps {
  diagnostic: Diagnostic;
  kind: "warning" | "error";
}

export function DiagnosticDetails({ diagnostic, kind }: DiagnosticDetailsProps) {
  const { t } = useTranslation();
  const titleKey = diagnosticTranslationKey(diagnostic.message_key, kind, diagnostic.code);
  const known = isKnownDiagnostic(diagnostic.message_key, kind, diagnostic.code);
  const details = diagnosticDetailEntries(diagnostic);

  return (
    <div className="min-w-0 flex-1">
      <div className="font-medium">{t(titleKey)}</div>
      <div className="mt-1 space-y-0.5 text-[11px] leading-4 text-current/80">
        <div>
          <span className="font-medium">{t("v3.diagnostics.details.code")}：</span>
          <code className="break-all">{diagnostic.code}</code>
        </div>
        {!known && (
          <div>
            <span className="font-medium">{t("v3.diagnostics.details.messageKey")}：</span>
            <code className="break-all">{diagnostic.message_key}</code>
          </div>
        )}
        {details.length > 0 ? details.map((detail, index) => (
          <div key={`${detail.labelKey}-${index}`}>
            <span className="font-medium">{t(detail.labelKey, detail.labelParams)}：</span>
            <span className="break-all">{detail.valueKey ? t(detail.valueKey, detail.valueParams) : detail.value}</span>
          </div>
        )) : (
          <div>{t("v3.diagnostics.details.noDetails")}</div>
        )}
      </div>
    </div>
  );
}
