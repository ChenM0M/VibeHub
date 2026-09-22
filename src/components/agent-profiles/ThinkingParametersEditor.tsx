import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export type ThinkingParameters = {
    reasoning_effort: string;
    thinking_mode: string;
    thinking_budget: string;
    effort_changed: boolean;
    thinking_changed: boolean;
};

export function ThinkingParametersEditor({ value, onChange, protocol, effortOptions = [], thinkingTypes = [], supportsEffort, supportsReasoning }: {
    value: ThinkingParameters; onChange: (value: ThinkingParameters) => void; protocol: string;
    effortOptions?: string[]; thinkingTypes?: string[]; supportsEffort: boolean | null; supportsReasoning: boolean | null;
}) {
    const { t } = useTranslation();
    const id = useId();
    const known = ['openai_chat_completions', 'openai_responses', 'anthropic_messages'].includes(protocol);
    const modes = supportsReasoning === false ? [] : thinkingTypes.length ? thinkingTypes : ['enabled', 'adaptive'];
    const availableModes = Array.from(new Set(['', ...modes, 'disabled', value.thinking_mode]));
    return <fieldset className="space-y-3 rounded-lg border p-3" disabled={!known}>
        <legend className="px-1 text-sm font-medium">{t('agentProfiles.thinkingParameters.title')}</legend>
        <p className="text-xs text-muted-foreground">{t(`agentProfiles.thinkingParameters.${known ? 'hint' : 'protocolRequired'}`)}</p>
        <div><Label htmlFor={id + 'thinking-effort'}>{t('agentProfiles.thinkingParameters.effort')}</Label>
            <Input id={id + 'thinking-effort'} value={value.reasoning_effort} list={id + 'thinking-effort-options'} placeholder={t('agentProfiles.thinkingParameters.inherit')}
                onChange={e => onChange({ ...value, reasoning_effort: e.target.value, effort_changed: true })} />
            <datalist id={id + 'thinking-effort-options'}>{effortOptions.map(option => <option key={option} value={option} />)}</datalist>
            {supportsEffort === false && <p className="text-xs text-muted-foreground">{t('agentProfiles.thinkingParameters.unsupportedEffort')}</p>}
        </div>
        {protocol === 'anthropic_messages' && <>
            <div><Label htmlFor={id + 'thinking-mode'}>{t('agentProfiles.thinkingParameters.mode')}</Label>
                <select id={id + 'thinking-mode'} className="mt-1 w-full rounded-md border bg-background p-2 text-sm" value={value.thinking_mode}
                    onChange={e => onChange({ ...value, thinking_mode: e.target.value, thinking_changed: true })}>
                    {availableModes.map(mode => <option key={mode} value={mode}>{t(`agentProfiles.thinkingParameters.${mode || 'inherit'}`, { defaultValue: mode })}</option>)}
                </select>
            </div>
            {value.thinking_mode === 'enabled' && <div><Label htmlFor={id + 'thinking-budget'}>{t('agentProfiles.thinkingParameters.budget')}</Label>
                <Input id={id + 'thinking-budget'} type="number" min={1024} step={1} value={value.thinking_budget}
                    onChange={e => onChange({ ...value, thinking_budget: e.target.value, thinking_changed: true })} />
            </div>}
        </>}
    </fieldset>;
}
