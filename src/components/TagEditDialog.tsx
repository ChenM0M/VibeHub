import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tag, TagCategory, TagConfig } from '@/types';
import { useTranslation } from 'react-i18next';

interface TagEditDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    tag?: Tag;
    onSave: (tag: Tag) => void;
}

export function TagEditDialog({ open, onOpenChange, tag, onSave }: TagEditDialogProps) {
    const { t } = useTranslation();
    const [name, setName] = useState('');
    const [color, setColor] = useState('#2EAADC');
    const [category, setCategory] = useState<TagCategory>('custom');
    const [config, setConfig] = useState<TagConfig>({});
    const [envVars, setEnvVars] = useState<{ key: string; value: string }[]>([]);

    useEffect(() => {
        if (tag) {
            setName(tag.name);
            setColor(tag.color);
            setCategory(tag.category);
            setConfig(tag.config || {});

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
            setEnvVars([]);
        }
    }, [tag, open]);

    const handleSave = () => {
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

        // Clean up empty fields
        if (!newConfig.executable) delete newConfig.executable;
        if (newConfig.args && newConfig.args.length === 0) delete newConfig.args;

        onSave({
            id: tag?.id || crypto.randomUUID(),
            name,
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
                                <div className="grid grid-cols-4 items-center gap-4">
                                    <label className="text-right text-sm font-medium">{t('tag.config.executable')}</label>
                                    <Input
                                        value={config.executable || ''}
                                        onChange={(e) => setConfig({ ...config, executable: e.target.value })}
                                        className="col-span-3"
                                        placeholder="e.g. code, npm, python"
                                    />
                                </div>
                                <div className="grid grid-cols-4 items-center gap-4">
                                    <label className="text-right text-sm font-medium">{t('tag.config.arguments')}</label>
                                    <Input
                                        value={config.args?.join(' ') || ''}
                                        onChange={(e) => setConfig({ ...config, args: e.target.value.split(' ') })}
                                        className="col-span-3"
                                        placeholder="Space separated args"
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
