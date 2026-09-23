import { useTranslation } from 'react-i18next';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import type { ModelLimitDraft } from './modelLimits';
export function ModelLimitsEditor({ value, onChange }: { value: ModelLimitDraft; onChange: (value: ModelLimitDraft) => void }) {
    const { t } = useTranslation();
    return <fieldset className="space-y-3 rounded-lg border p-3"><legend className="px-1 text-sm font-medium">{t('agentProfiles.limits.title')}</legend>
        <p className="text-xs text-muted-foreground">{t('agentProfiles.limits.hint')}</p>
        <div className="grid gap-3 sm:grid-cols-3">{(['context', 'input', 'output'] as const).map(key => <label key={key} className="text-sm">{t(`agentProfiles.limits.${key}`)}
            <Input type="number" min={1} step={1} value={value[key]} placeholder={t('agentProfiles.limits.inherit')} onChange={e => onChange({ ...value, [key]: e.target.value })} />
        </label>)}</div>
        <Button type="button" variant="outline" size="sm" onClick={() => onChange({ context: '', input: '', output: '' })}>{t('agentProfiles.limits.reset')}</Button>
    </fieldset>;
}
