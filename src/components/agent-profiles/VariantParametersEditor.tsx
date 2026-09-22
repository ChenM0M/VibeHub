import { useTranslation } from 'react-i18next';
import { Input } from '@/components/ui/input';
import { ThinkingParametersEditor } from './ThinkingParametersEditor';
import { parseVariantObject } from './variantValues';

export function VariantParametersEditor({ name, text, protocol, onChange }: { name: string; text: string; protocol: string; onChange: (text: string) => void }) {
    const { t } = useTranslation();
    let object: Record<string, unknown> | null = null;
    try { object = parseVariantObject(text); } catch { /* Keep invalid raw text editable. */ }
    const thinking = object?.thinking && typeof object.thinking === 'object' ? object.thinking as Record<string, unknown> : {};
    const effortKey = protocol === 'anthropic_messages' ? 'effort' : 'reasoningEffort';
    const update = (value: Record<string, unknown>) => onChange(JSON.stringify(value, null, 2));
    return <section className="space-y-3 rounded-lg border p-3">
        <h4 className="text-sm font-medium">{name}</h4>
        {object && <>
            <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={object.disabled === true} onChange={e => update({ ...object, disabled: e.target.checked })} />{t('agentProfiles.variants.disabled')}</label>
            <ThinkingParametersEditor protocol={protocol} supportsEffort={null} supportsReasoning={null}
                value={{ reasoning_effort: String(object[effortKey] ?? ''), thinking_mode: String(thinking.type ?? ''), thinking_budget: String(thinking.budgetTokens ?? ''), effort_changed: false, thinking_changed: false }}
                onChange={value => {
                    const next = { ...object };
                    if (value.effort_changed) { if (value.reasoning_effort.trim()) next[effortKey] = value.reasoning_effort.trim(); else delete next[effortKey]; }
                    if (value.thinking_changed) {
                        if (!value.thinking_mode) delete next.thinking;
                        else {
                            const nextThinking: Record<string, unknown> = { ...thinking, type: value.thinking_mode };
                            if (value.thinking_mode === 'enabled' && value.thinking_budget) nextThinking.budgetTokens = Number(value.thinking_budget);
                            else delete nextThinking.budgetTokens;
                            next.thinking = nextThinking;
                        }
                    }
                    update(next);
                }} />
            <label className="block text-sm">{t('agentProfiles.variants.temperature')}<Input type="number" step="any" value={typeof object.temperature === 'number' ? object.temperature : ''} onChange={e => {
                const next = { ...object }; if (e.target.value === '') delete next.temperature; else next.temperature = Number(e.target.value); update(next);
            }} /></label>
        </>}
        <details open={!object}><summary className="cursor-pointer text-xs text-muted-foreground">{t('agentProfiles.variants.allParameters')}</summary>
            <textarea aria-label={t('agentProfiles.variants.jsonLabel', { name })} className="mt-2 min-h-28 w-full rounded-md border bg-background p-2 font-mono text-xs" value={text} onChange={e => onChange(e.target.value)} />
        </details>
        {!object && <p role="alert" className="text-xs text-destructive">{t('agentProfiles.variants.invalid')}</p>}
    </section>;
}
