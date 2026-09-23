import { useTranslation } from 'react-i18next';

export function ModelModalitiesEditor({ input, output, onChange }: { input: string[] | null; output: string[] | null; onChange: (direction: 'input' | 'output', values: string[] | null) => void }) {
    const { t } = useTranslation();
    return <fieldset className="space-y-3 rounded-lg border p-3"><legend className="px-1 text-sm font-medium">{t('agentProfiles.modalities.title')}</legend>
        <p className="text-xs text-muted-foreground">{t('agentProfiles.modalities.hint')}</p>
        {(['input', 'output'] as const).map(direction => {
            const values = direction === 'input' ? input : output;
            return <div key={direction} className="space-y-2">
                <label className="block text-sm">{t(`agentProfiles.modalities.${direction}`)}
                    <select className="ml-2 rounded-md border bg-background p-1" value={values === null ? 'inherit' : 'declare'} onChange={e => onChange(direction, e.target.value === 'inherit' ? null : [])}>
                        <option value="inherit">{t('agentProfiles.modalities.inherit')}</option><option value="declare">{t('agentProfiles.modalities.declare')}</option>
                    </select>
                </label>
                {values !== null && <div className="flex flex-wrap gap-3">{Array.from(new Set(['text', 'image', 'audio', 'video', 'pdf', ...values])).map(type => <label key={type} className="flex items-center gap-1 text-sm">
                    <input type="checkbox" checked={values.includes(type)} onChange={e => onChange(direction, e.target.checked ? [...values, type] : values.filter(v => v !== type))} />{t(`agentProfiles.modalities.${type}`, { defaultValue: type })}
                </label>)}</div>}
                {values?.length === 0 && <p className="text-xs text-muted-foreground">{t('agentProfiles.modalities.empty')}</p>}
            </div>;
        })}
    </fieldset>;
}
