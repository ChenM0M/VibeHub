import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tag, TagCategory, TagConfig } from '@/types';
import { useTranslation } from 'react-i18next';
import { isLaunchableCategory, normalizeLaunchInput } from '@/lib/tagLaunch';

interface TagEditDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    tag?: Tag;
    onSave: (tag: Tag) => void;
}

type ClientPlatform = 'macos' | 'windows' | 'linux' | 'unknown';

const detectClientPlatform = (): ClientPlatform => {
    if (typeof navigator === 'undefined') return 'unknown';
    const userAgent = navigator.userAgent.toLowerCase();
    if (userAgent.includes('mac')) return 'macos';
    if (userAgent.includes('windows')) return 'windows';
    if (userAgent.includes('linux')) return 'linux';
    return 'unknown';
};

const quoteForDisplay = (value: string) => (/\s/.test(value) ? `"${value}"` : value);

const getTerminalOptions = (platform: ClientPlatform) => {
    if (platform === 'windows') {
        return [
            { value: '', label: 'Auto (Windows default)' },
            { value: 'WindowsTerminal', label: 'Windows Terminal' },
            { value: 'PowerShell', label: 'PowerShell' },
            { value: 'CommandPrompt', label: 'Command Prompt' },
        ];
    }

    if (platform === 'macos') {
        return [
            { value: '', label: 'Auto (Terminal.app)' },
            { value: 'Terminal', label: 'Terminal.app' },
            { value: 'iTerm', label: 'iTerm.app' },
            { value: 'Warp', label: 'Warp.app' },
        ];
    }

    return [{ value: '', label: 'Auto' }];
};

export function TagEditDialog({ open, onOpenChange, tag, onSave }: TagEditDialogProps) {
    const { t } = useTranslation();
    const [name, setName] = useState('');
    const [color, setColor] = useState('#2EAADC');
    const [category, setCategory] = useState<TagCategory>('custom');
    const [config, setConfig] = useState<TagConfig>({});
    const [executableInput, setExecutableInput] = useState('');
    const [argsInput, setArgsInput] = useState('');
    const [error, setError] = useState<string | null>(null);
    const [envVars, setEnvVars] = useState<{ key: string; value: string }[]>([]);
    const platform = detectClientPlatform();
    const terminalOptions = getTerminalOptions(platform);
    const terminalValue = terminalOptions.some(option => option.value === (config.terminal || ''))
        ? config.terminal || ''
        : '';

    useEffect(() => {
        setError(null);
        if (tag) {
            setName(tag.name);
            setColor(tag.color);
            setCategory(tag.category);
            setConfig(tag.config || {});
            setExecutableInput(tag.config?.executable || '');
            setArgsInput((tag.config?.args || []).map(quoteForDisplay).join(' '));

            if (tag.config?.env) {
                setEnvVars(Object.entries(tag.config.env).map(([key, value]) => ({ key, value })));
            } else {
                setEnvVars([]);
            }
        } else {
            setName('');
            setColor('#2EAADC');
            setCategory('custom');
            setConfig({});
            setExecutableInput('');
            setArgsInput('');
            setEnvVars([]);
        }
    }, [tag, open]);

    const launchable = isLaunchableCategory(category);
    const normalizedLaunch = normalizeLaunchInput(executableInput, argsInput);

    const handleSave = () => {
        const trimmedName = name.trim();
        if (!trimmedName) {
            setError(t('tag.errors.nameRequired'));
            return;
        }
        if (launchable && !normalizedLaunch.executable) {
            setError(t('tag.errors.executableRequired'));
            return;
        }

        const newConfig: TagConfig = { ...config };

        // Process env vars
        if (envVars.length > 0) {
            newConfig.env = envVars.reduce((acc, { key, value }) => {
                if (key) acc[key] = value;
                return acc;
            }, {} as Record<string, string>);
        } else {
            delete newConfig.env;
        }

        if (launchable) {
            newConfig.executable = normalizedLaunch.executable;
            if (normalizedLaunch.args.length > 0) {
                newConfig.args = normalizedLaunch.args;
            } else {
                delete newConfig.args;
            }
        } else {
            delete newConfig.executable;
            delete newConfig.args;
        }
        if (!newConfig.terminal) delete newConfig.terminal;

        setError(null);
        onSave({
            id: tag?.id || crypto.randomUUID(),
            name: trimmedName,
            color,
            category,
            config: Object.keys(newConfig).length > 0 ? newConfig : undefined,
        });
        onOpenChange(false);
    };

    const addEnvVar = () => setEnvVars([...envVars, { key: '', value: '' }]);
    const removeEnvVar = (index: number) => setEnvVars(envVars.filter((_, i) => i !== index));
    const updateEnvVar = (index: number, field: 'key' | 'value', value: string) => {
        const newEnvVars = [...envVars];
        newEnvVars[index][field] = value;
        setEnvVars(newEnvVars);
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50" />
                <Dialog.Content className="fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 sm:rounded-lg">
                    <div className="flex flex-col space-y-1.5 text-center sm:text-left">
                        <Dialog.Title className="text-lg font-semibold leading-none tracking-tight">
                            {tag ? t('tag.editTitle') : t('tag.createTitle')}
                        </Dialog.Title>
                    </div>

                    <div className="grid gap-4 py-4">
                        <div className="grid grid-cols-4 items-center gap-4">
                            <label className="text-right text-sm font-medium">{t('tag.name')}</label>
                            <Input value={name} onChange={(e) => setName(e.target.value)} className="col-span-3" />
                        </div>
                        <div className="grid grid-cols-4 items-center gap-4">
                            <label className="text-right text-sm font-medium">{t('tag.color')}</label>
                            <div className="col-span-3 flex gap-2">
                                <Input type="color" value={color} onChange={(e) => setColor(e.target.value)} className="w-12 p-1 h-9" />
                                <Input value={color} onChange={(e) => setColor(e.target.value)} className="flex-1" />
                            </div>
                        </div>
                        <div className="grid grid-cols-4 items-center gap-4">
                            <label className="text-right text-sm font-medium">{t('tag.category')}</label>
                            <select
                                className="col-span-3 flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                value={category}
                                onChange={(e) => setCategory(e.target.value as TagCategory)}
                            >
                                <option value="custom">{t('tag.categories.custom')}</option>
                                <option value="workspace">{t('tag.categories.workspace')}</option>
                                <option value="ide">{t('tag.categories.ide')}</option>
                                <option value="cli">{t('tag.categories.cli')}</option>
                                <option value="environment">{t('tag.categories.environment')}</option>
                                <option value="startup">{t('tag.categories.startup')}</option>
                            </select>
                        </div>

                        {/* Configuration Fields based on Category */}
                        {(category === 'ide' || category === 'cli' || category === 'startup') && (
                            <>
                                {category === 'cli' && (
                                    <div className="grid grid-cols-4 items-center gap-4">
                                        <label className="text-right text-sm font-medium">Terminal</label>
                                        <select
                                            className="col-span-3 flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                            value={terminalValue}
                                            onChange={(e) => {
                                                const terminal = e.target.value || undefined;
                                                setConfig({ ...config, terminal });
                                            }}
                                        >
                                            {terminalOptions.map(option => (
                                                <option key={option.value || 'auto'} value={option.value}>
                                                    {option.label}
                                                </option>
                                            ))}
                                        </select>
                                        {config.terminal && terminalValue === '' && (
                                            <div className="col-start-2 col-span-3 text-xs text-muted-foreground">
                                                Current terminal "{config.terminal}" is preserved for another platform.
                                            </div>
                                        )}
                                    </div>
                                )}
                                <div className="grid grid-cols-4 items-center gap-4">
                                    <label className="text-right text-sm font-medium">{t('tag.config.executable')}</label>
                                    <Input
                                        value={executableInput}
                                        onChange={(e) => setExecutableInput(e.target.value)}
                                        className="col-span-3"
                                        placeholder={category === 'cli' ? 'e.g. opencode, claude, amp' : 'e.g. code, npm, python'}
                                    />
                                </div>
                                <div className="grid grid-cols-4 items-center gap-4">
                                    <label className="text-right text-sm font-medium">{t('tag.config.arguments')}</label>
                                    <Input
                                        value={argsInput}
                                        onChange={(e) => setArgsInput(e.target.value)}
                                        className="col-span-3"
                                        placeholder={'--settings "/path/to/settings.json"'}
                                    />
                                </div>
                            </>
                        )}

                        {(category === 'environment' || category === 'ide' || category === 'cli' || category === 'startup') && (
                            <div className="space-y-2 border-t pt-4 mt-2">
                                <div className="flex items-center justify-between">
                                    <label className="text-sm font-medium">{t('tag.config.envVars')}</label>
                                    <Button type="button" variant="outline" size="sm" onClick={addEnvVar}>
                                        <Plus className="h-3 w-3 mr-1" /> {t('tag.config.addEnv')}
                                    </Button>
                                </div>
                                <div className="space-y-2 max-h-40 overflow-y-auto pr-1">
                                    {envVars.map((env, index) => (
                                        <div key={index} className="flex gap-2">
                                            <Input
                                                placeholder="KEY"
                                                value={env.key}
                                                onChange={(e) => updateEnvVar(index, 'key', e.target.value)}
                                                className="flex-1"
                                            />
                                            <Input
                                                placeholder="VALUE"
                                                value={env.value}
                                                onChange={(e) => updateEnvVar(index, 'value', e.target.value)}
                                                className="flex-1"
                                            />
                                            <Button type="button" variant="ghost" size="icon" onClick={() => removeEnvVar(index)}>
                                                <Trash2 className="h-4 w-4 text-destructive" />
                                            </Button>
                                        </div>
                                    ))}
                                    {envVars.length === 0 && (
                                        <div className="text-xs text-muted-foreground text-center py-2">
                                            No environment variables configured
                                        </div>
                                    )}
                                </div>
                            </div>
                        )}
                    </div>

                    {error && (
                        <div role="alert" className="text-sm text-destructive break-words">
                            {error}
                        </div>
                    )}

                    <div className="flex justify-end gap-3">
                        <Button variant="outline" onClick={() => onOpenChange(false)}>{t('common.cancel')}</Button>
                        <Button onClick={handleSave}>{t('common.save')}</Button>
                    </div>

                    <Dialog.Close asChild>
                        <button className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground">
                            <X className="h-4 w-4" />
                            <span className="sr-only">Close</span>
                        </button>
                    </Dialog.Close>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
