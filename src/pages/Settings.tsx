import React, { useState } from 'react';
import { useAppStore } from '@/stores/appStore';
import { Button } from '@/components/ui/button';
import { Trash2, Plus, FolderOpen, Tags, Settings as SettingsIcon, Sun, Moon, Monitor, FileDown, FileUp, CheckCircle2, AlertCircle } from 'lucide-react';
import { open, save } from '@tauri-apps/plugin-dialog';
import { SettingsImportResult, Tag } from '@/types';
import { TagEditDialog } from '@/components/TagEditDialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useTranslation } from 'react-i18next';
import { tauriApi } from '@/services/tauri';

export function Settings() {
    const { t, i18n } = useTranslation();
    const { config, refreshConfig, addWorkspace, removeWorkspace, addTag, updateTag, deleteTag, setTheme } = useAppStore();
    const [isScanning, setIsScanning] = useState(false);
    const [isTagDialogOpen, setIsTagDialogOpen] = useState(false);
    const [editingTag, setEditingTag] = useState<Tag | undefined>(undefined);
    const [transferBusy, setTransferBusy] = useState(false);
    const [transferError, setTransferError] = useState<string | null>(null);
    const [transferSuccess, setTransferSuccess] = useState<string | null>(null);
    const [importResult, setImportResult] = useState<SettingsImportResult | null>(null);

    const handleAddWorkspace = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
            });
            if (selected && typeof selected === 'string') {
                setIsScanning(true);
                const name = selected.split(/[\\/]/).pop() || 'Workspace';
                await addWorkspace(name, selected, true);
            }
        } catch (error) {
            console.error(error);
        } finally {
            setIsScanning(false);
        }
    };

    const handleExportSettings = async () => {
        setTransferError(null);
        setTransferSuccess(null);
        setImportResult(null);
        try {
            const selected = await save({
                defaultPath: 'vibehub-settings.json',
                filters: [{ name: 'JSON', extensions: ['json'] }],
            });
            if (!selected) return;

            setTransferBusy(true);
            await tauriApi.exportSettingsBundle(selected);
            setTransferSuccess(t('settings.transfer.exportSuccess'));
        } catch (error) {
            setTransferError((error as Error).message || String(error));
        } finally {
            setTransferBusy(false);
        }
    };

    const handleImportSettings = async () => {
        setTransferError(null);
        setTransferSuccess(null);
        setImportResult(null);
        try {
            const selected = await open({
                directory: false,
                multiple: false,
                filters: [{ name: 'JSON', extensions: ['json'] }],
            });
            if (!selected || typeof selected !== 'string') return;

            setTransferBusy(true);
            const result = await tauriApi.importSettingsBundle(selected);
            await refreshConfig();
            setImportResult(result);
            setTransferSuccess(t('settings.transfer.importSuccess', {
                added: result.tags_added,
                updated: result.tags_updated,
                providers: result.gateway_providers,
            }));
        } catch (error) {
            setTransferError((error as Error).message || String(error));
        } finally {
            setTransferBusy(false);
        }
    };

    if (!config) return null;

    return (
        <div className="container mx-auto max-w-4xl py-8 space-y-8 animate-slide-in">
            <div className="flex items-center justify-between">
                <div>
                    <h1 className="text-3xl font-bold tracking-tight">{t('settings.title')}</h1>
                    <p className="text-muted-foreground mt-1">
                        {t('settings.subtitle')}
                    </p>
                </div>
            </div>

            <Tabs defaultValue="workspaces" className="w-full">
                <TabsList className="grid w-full grid-cols-3 mb-8">
                    <TabsTrigger value="workspaces" className="flex items-center gap-2">
                        <FolderOpen className="h-4 w-4" />
                        {t('common.workspaces')}
                    </TabsTrigger>
                    <TabsTrigger value="tags" className="flex items-center gap-2">
                        <Tags className="h-4 w-4" />
                        {t('common.tags')}
                    </TabsTrigger>
                    <TabsTrigger value="general" className="flex items-center gap-2">
                        <SettingsIcon className="h-4 w-4" />
                        {t('common.general')}
                    </TabsTrigger>
                </TabsList>

                <TabsContent value="workspaces" className="space-y-6">
                    <div className="flex items-center justify-between">
                        <div>
                            <h2 className="text-lg font-semibold">{t('settings.workspaces.title')}</h2>
                            <p className="text-sm text-muted-foreground">
                                {t('settings.workspaces.subtitle')}
                            </p>
                        </div>
                        <Button onClick={handleAddWorkspace} disabled={isScanning}>
                            <Plus className={`h-4 w-4 mr-2 ${isScanning ? 'animate-spin' : ''}`} />
                            {t('settings.workspaces.add')}
                        </Button>
                    </div>

                    <div className="grid gap-4">
                        {config.workspaces.map((workspace) => (
                            <div
                                key={workspace.id}
                                className="flex items-center justify-between p-4 rounded-lg border bg-card text-card-foreground shadow-sm"
                            >
                                <div className="flex items-center gap-4">
                                    <div className="p-2 rounded-md bg-primary/10 text-primary">
                                        <FolderOpen className="h-5 w-5" />
                                    </div>
                                    <div>
                                        <h3 className="font-medium">{workspace.name}</h3>
                                        <p className="text-sm text-muted-foreground">{workspace.path}</p>
                                    </div>
                                </div>
                                <div className="flex items-center gap-2">
                                    <Button
                                        variant="ghost"
                                        size="icon"
                                        className="text-destructive hover:text-destructive hover:bg-destructive/10"
                                        onClick={() => removeWorkspace(workspace.id)}
                                    >
                                        <Trash2 className="h-4 w-4" />
                                    </Button>
                                </div>
                            </div>
                        ))}

                        {config.workspaces.length === 0 && (
                            <div className="text-center py-12 border border-dashed rounded-lg text-muted-foreground">
                                <FolderOpen className="h-12 w-12 mx-auto mb-4 opacity-50" />
                                <p>{t('settings.workspaces.noWorkspaces')}</p>
                            </div>
                        )}
                    </div>
                </TabsContent>

                <TabsContent value="tags" className="space-y-6">
                    <div className="flex items-center justify-between">
                        <div>
                            <h2 className="text-lg font-semibold">{t('settings.tags.title')}</h2>
                            <p className="text-sm text-muted-foreground">
                                {t('settings.tags.subtitle')}
                            </p>
                        </div>
                        <Button onClick={() => {
                            setEditingTag(undefined);
                            setIsTagDialogOpen(true);
                        }}>
                            <Plus className="h-4 w-4 mr-2" />
                            {t('settings.tags.add')}
                        </Button>
                    </div>

                    <div className="grid gap-3 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
                        {config.tags.map((tag) => (
                            <div
                                key={tag.id}
                                className="flex items-center justify-between p-3 rounded-lg border bg-card hover:shadow-md transition-shadow cursor-pointer group"
                                onClick={() => {
                                    setEditingTag(tag);
                                    setIsTagDialogOpen(true);
                                }}
                            >
                                <div className="flex items-center gap-3">
                                    <div
                                        className="w-4 h-4 rounded-full shadow-sm ring-2 ring-offset-2 ring-offset-background"
                                        style={{ backgroundColor: tag.color, '--tw-ring-color': tag.color } as React.CSSProperties}
                                    />
                                    <div>
                                        <span className="font-medium">{tag.name}</span>
                                        <p className="text-xs text-muted-foreground capitalize">{tag.category}</p>
                                    </div>
                                </div>
                                <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:text-destructive hover:bg-destructive/10"
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        deleteTag(tag.id);
                                    }}
                                >
                                    <Trash2 className="h-4 w-4" />
                                </Button>
                            </div>
                        ))}
                    </div>
                </TabsContent>

                <TabsContent value="general" className="space-y-6">
                    <div className="space-y-4">
                        <div>
                            <h3 className="text-lg font-medium">{t('settings.appearance.title')}</h3>
                            <p className="text-sm text-muted-foreground">
                                {t('settings.appearance.subtitle')}
                            </p>
                        </div>
                        <div className="flex items-center gap-4">
                            <Button
                                variant={config.theme === 'light' ? 'default' : 'outline'}
                                onClick={() => setTheme('light')}
                                className="w-32"
                            >
                                <Sun className="mr-2 h-4 w-4" />
                                {t('settings.appearance.light')}
                            </Button>
                            <Button
                                variant={config.theme === 'dark' ? 'default' : 'outline'}
                                onClick={() => setTheme('dark')}
                                className="w-32"
                            >
                                <Moon className="mr-2 h-4 w-4" />
                                {t('settings.appearance.dark')}
                            </Button>
                            <Button
                                variant={config.theme === 'auto' ? 'default' : 'outline'}
                                onClick={() => setTheme('auto')}
                                className="w-32"
                            >
                                <Monitor className="mr-2 h-4 w-4" />
                                {t('settings.appearance.auto')}
                            </Button>
                        </div>
                    </div>

                    <div className="space-y-4">
                        <div>
                            <h3 className="text-lg font-medium">{t('settings.appearance.language')}</h3>
                            <p className="text-sm text-muted-foreground">
                                {t('settings.appearance.languageSubtitle')}
                            </p>
                        </div>
                        <div className="flex items-center gap-4">
                            <Button
                                variant={i18n.language.startsWith('en') ? 'default' : 'outline'}
                                onClick={() => i18n.changeLanguage('en')}
                                className="w-32"
                            >
                                English
                            </Button>
                            <Button
                                variant={i18n.language === 'zh' || i18n.language === 'zh-CN' ? 'default' : 'outline'}
                                onClick={() => i18n.changeLanguage('zh')}
                                className="w-32"
                            >
                                简体中文
                            </Button>
                            <Button
                                variant={i18n.language === 'zh-TW' || i18n.language === 'zh-HK' ? 'default' : 'outline'}
                                onClick={() => i18n.changeLanguage('zh-TW')}
                                className="w-32"
                            >
                                繁體中文
                            </Button>
                        </div>
                    </div>

                    <div className="space-y-4 border-t pt-6">
                        <div>
                            <h3 className="text-lg font-medium">{t('settings.transfer.title')}</h3>
                            <p className="text-sm text-muted-foreground">
                                {t('settings.transfer.subtitle')}
                            </p>
                        </div>
                        <div className="flex flex-wrap items-center gap-3">
                            <Button variant="outline" onClick={handleExportSettings} disabled={transferBusy}>
                                <FileDown className="mr-2 h-4 w-4" />
                                {t('settings.transfer.export')}
                            </Button>
                            <Button onClick={handleImportSettings} disabled={transferBusy}>
                                <FileUp className="mr-2 h-4 w-4" />
                                {t('settings.transfer.import')}
                            </Button>
                        </div>
                        <p className="text-xs text-muted-foreground">
                            {t('settings.transfer.note')}
                        </p>

                        {(transferSuccess || transferError || importResult) && (
                            <div className="rounded-md border bg-card p-4 text-sm space-y-3">
                                {transferSuccess && (
                                    <div className="flex items-start gap-2 text-green-600 dark:text-green-400">
                                        <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0" />
                                        <span>{transferSuccess}</span>
                                    </div>
                                )}
                                {transferError && (
                                    <div className="flex items-start gap-2 text-destructive">
                                        <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
                                        <span>{transferError}</span>
                                    </div>
                                )}
                                {importResult && (
                                    <div className="space-y-2">
                                        <div className="text-muted-foreground">
                                            {t('settings.transfer.sourceTarget', {
                                                source: importResult.source_system,
                                                target: importResult.target_system,
                                            })}
                                        </div>
                                        {importResult.adjustments.length > 0 ? (
                                            <div className="space-y-2">
                                                <div className="font-medium">{t('settings.transfer.adjustments')}</div>
                                                <div className="max-h-48 overflow-y-auto space-y-2 pr-1">
                                                    {importResult.adjustments.map((adjustment, index) => (
                                                        <div key={`${adjustment.scope}-${adjustment.item_id || index}-${adjustment.field}`} className="rounded border bg-muted/30 p-2">
                                                            <div className="font-medium">
                                                                {adjustment.item_name || adjustment.scope} · {adjustment.field}
                                                            </div>
                                                            <div className="text-xs text-muted-foreground">
                                                                {adjustment.before || t('common.none')} → {adjustment.after || t('common.none')}
                                                            </div>
                                                            <div className="text-xs text-muted-foreground">
                                                                {adjustment.reason}
                                                            </div>
                                                        </div>
                                                    ))}
                                                </div>
                                            </div>
                                        ) : (
                                            <div className="text-muted-foreground">{t('settings.transfer.noAdjustments')}</div>
                                        )}
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                </TabsContent>
            </Tabs>

            <TagEditDialog
                open={isTagDialogOpen}
                onOpenChange={(open) => {
                    setIsTagDialogOpen(open);
                    if (!open) setEditingTag(undefined);
                }}
                tag={editingTag}
                onSave={async (tag) => {
                    if (editingTag) {
                        await updateTag(tag);
                    } else {
                        await addTag(tag);
                    }
                    setIsTagDialogOpen(false);
                    setEditingTag(undefined);
                }}
            />
        </div>
    );
}
