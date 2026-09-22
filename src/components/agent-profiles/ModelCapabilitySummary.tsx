import { useTranslation } from 'react-i18next';
import type { ModelProfile } from '@/v3/contracts/generated/agent-profile';
export function ModelCapabilitySummary({ model }: { model: ModelProfile }) {
    const { t } = useTranslation();
    const states = { tools: model.supports_tools ?? null, images: model.modalities?.input == null ? null : model.modalities.input.includes('image'), reasoning: model.thinking.supports_reasoning };
    return <div className="mt-2 space-y-1 pl-6 text-[11px] text-muted-foreground" aria-label={t('agentProfiles.capabilityAssessment.title')}>
        <div className="flex flex-wrap gap-x-3">{Object.entries(states).map(([name, state]) => <span key={name}>{t(`agentProfiles.capabilityAssessment.${name}`)}: {t(`agentProfiles.capabilityAssessment.${state === null ? 'unknown' : state ? 'yes' : 'no'}`)}</span>)}</div>
        <p>{t('agentProfiles.capabilityAssessment.hint')}</p>
    </div>;
}
