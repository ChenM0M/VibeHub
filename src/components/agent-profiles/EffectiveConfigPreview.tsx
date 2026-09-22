import { useTranslation } from 'react-i18next';
import type { AgentProfileDocument, RuntimeTarget } from '@/v3/contracts/generated/agent-profile';
import { knownConfiguration, launchCommand } from './effectiveConfig';

export function EffectiveConfigPreview({ profile, saved, target, dirty }: { profile: AgentProfileDocument; saved: AgentProfileDocument | null; target: RuntimeTarget | null; dirty: boolean }) {
    const { t } = useTranslation();
    const rows = knownConfiguration(profile);
    const savedRows = saved ? knownConfiguration(saved) : [];
    const launch = launchCommand(profile, target);
    const display = (value: unknown) => value === null || value === undefined ? t('agentProfiles.preview.inherited') : typeof value === 'string' ? value : JSON.stringify(value);
    return <details className="mt-4 rounded-lg border p-4">
        <summary className="cursor-pointer text-sm font-medium">{t('agentProfiles.preview.title')}{dirty ? ` · ${t('agentProfiles.preview.draft')}` : ''}</summary>
        <p className="mt-3 text-xs text-muted-foreground">{t('agentProfiles.preview.scope')}</p>
        <p className="mt-2 break-all font-mono text-xs">{profile.source.path.display}</p>
        <div className="mt-3 overflow-x-auto"><table className="w-full text-left text-xs"><thead><tr>{['field', 'value', 'source'].map(key => <th key={key} className="border-b p-2">{t(`agentProfiles.preview.${key}`)}</th>)}</tr></thead><tbody>
            {rows.map(row => {
                const changed = dirty && JSON.stringify(row) !== JSON.stringify(savedRows.find(savedRow => savedRow.key === row.key));
                return <tr key={row.key}><td className="border-b p-2">{t(`agentProfiles.preview.fields.${row.key}`)}{changed && <span className="ml-1 text-amber-600">{t('agentProfiles.preview.draft')}</span>}</td>
                    <td className="max-w-80 break-all border-b p-2 font-mono">{display(row.value)}{row.source === 'variant' && <div className="mt-1 text-muted-foreground">{t('agentProfiles.preview.overrides', { value: display(row.overridden) })}</div>}</td>
                    <td className="border-b p-2">{t(`agentProfiles.preview.origins.${row.source}`)}{row.variant ? `: ${row.variant}` : ''}</td></tr>;
            })}
        </tbody></table></div>
        {profile.agent === 'opencode' && <p className="mt-3 text-xs text-muted-foreground">{t('agentProfiles.preview.layers')}</p>}
        <p className="mt-3 text-xs font-medium">{t('agentProfiles.preview.launch', { shell: launch.shell === 'powershell' ? 'PowerShell' : 'sh / bash / zsh' })}</p>
        <pre className="mt-1 select-text overflow-auto whitespace-pre-wrap break-all rounded bg-muted/40 p-2 text-xs">{launch.command || t('agentProfiles.preview.runtimePathMismatch')}</pre>
        {dirty && <p className="mt-2 text-xs text-amber-600">{t('agentProfiles.preview.saveFirst')}</p>}
    </details>;
}
