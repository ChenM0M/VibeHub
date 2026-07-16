import React, { useEffect, useState } from 'react';
import { useAppStore } from '@/stores/appStore';
import { Button } from '@/components/ui/button';
import { Trash2, Plus, FolderOpen, Tags, Settings as SettingsIcon, Sun, Moon, Monitor, FileDown, FileUp, CheckCircle2, AlertCircle, HardDrive, FolderCog, RotateCcw, ExternalLink, Info } from 'lucide-react';
import { open, save } from '@tauri-apps/plugin-dialog';
import { SettingsImportResult, StorageInfo, Tag } from '@/types';
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
    const [storageInfo, setStorageInfo] = useState<StorageInfo | null>(null);
    const [storageBusy, setStorageBusy] = useState(false);
    const [storageMessage, setStorageMessage] = useState<{ kind: 'success' | 'error'; text: string } | null>(null);

    useEffect(() => {
        tauriApi.getStorageInfo()
            .then(setStorageInfo)
            .catch((e) => setStorageMessage({ kind: 'error', text: (e as Error).message || String(e) }));
    }, []);

    const handleOpenStorageDir = async () => {
        if (!storageInfo) return;
        try {
            await tauriApi.openInExplorer(storageInfo.active_dir);
        } catch (e) {
            setStorageMessage({ kind: 'error', text: t('settings.storage.openError', { message: (e as Error).message || String(e) }) });
        }
    };

    const handlePickCustomDir = async () => {
        setStorageMessage(null);
        try {
            const selected = await open({ directory: true, multiple: false });
            if (!selected || typeof selected !== 'string') return;
            // Confirm so the user knows we won't auto-migrate existing data.
            // We use the native confirm dialog since the user explicitly
            // asked for "popup reminder, with dismiss" — the per-action
            // confirmation here is intentionally lightweight and not
            // dismiss-able (dismiss applies only to the persistent notice
            // banner once the change is applied).
            // eslint-disable-next-line no-restricted-globals
            if (!confirm(t('settings.storage.confirmSwitchBody'))) return;

            setStorageBusy(true);
            const next = await tauriApi.setCustomDataDir(selected);
            setStorageInfo(next);
            setStorageMessage({ kind: 'success', text: t('settings.storage.saveSuccess') });
        } catch (e) {
            setStorageMessage({ kind: 'error', text: t('settings.storage.saveError', { message: (e as Error).message || String(e) }) });
        } finally {
            setStorageBusy(false);
        }
    };

    const handleClearCustomDir = async () => {
        setStorageMessage(null);
        setStorageBusy(true);
        try {
            const next = await tauriApi.clearCustomDataDir();
            setStorageInfo(next);
            setStorageMessage({ kind: 'success', text: t('settings.storage.saveSuccess') });
        } catch (e) {
            setStorageMessage({ kind: 'error', text: t('settings.storage.saveError', { message: (e as Error).message || String(e) }) });
        } finally {
            setStorageBusy(false);
        }
    };

    const handleDismissMigrationNotice = async () => {
        try {
            const next = await tauriApi.dismissStorageMigrationNotice();
            setStorageInfo(next);
        } catch (e) {
            setStorageMessage({ kind: 'error', text: (e as Error).message || String(e) });
        }
    };

    const handleDismissCustomDirNotice = async () => {
        try {
            const next = await tauriApi.dismissStorageCustomDirNotice();
            setStorageInfo(next);
        } catch (e) {
            setStorageMessage({ kind: 'error', text: (e as Error).message || String(e) });
        }
    };

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
                            <h3 className="text-lg font-medium flex items-center gap-2">
                                <HardDrive className="h-4 w-4" />
                                {t('settings.storage.title')}
                            </h3>
                            <p className="text-sm text-muted-foreground">
                                {t('settings.storage.subtitle')}
                            </p>
                        </div>

                        {storageInfo && (
                            <div className="rounded-md border bg-card p-4 space-y-3 text-sm">
                                <div className="flex flex-col gap-1">
                                    <div className="text-xs uppercase tracking-wide text-muted-foreground">
                                        {t('settings.storage.activeDir')}
                                    </div>
                                    <div className="font-mono break-all">{storageInfo.active_dir}</div>
                                    <div className="flex items-center gap-2 mt-1">
                                        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-primary/10 text-primary">
                                            {t('settings.storage.sourceLabel')}: {t(`settings.storage.source.${storageInfo.source}`)}
                                        </span>
                                        <span className="text-xs text-muted-foreground">
                                            {t(`settings.storage.sourceHint.${storageInfo.source}`)}
                                        </span>
                                    </div>
                                </div>

                                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2 border-t">
                                    <div>
                                        <div className="text-xs uppercase tracking-wide text-muted-foreground">
                                            {t('settings.storage.defaultDir')}
                                        </div>
                                        <div className="font-mono text-xs break-all">{storageInfo.default_dir}</div>
                                    </div>
                                    <div>
                                        <div className="text-xs uppercase tracking-wide text-muted-foreground">
                                            {t('settings.storage.portableDir')}
                                        </div>
                                        <div className="font-mono text-xs break-all">
                                            {storageInfo.portable_dir || '—'}
                                        </div>
                                        <div className={`text-xs mt-0.5 ${storageInfo.portable_available ? 'text-green-600 dark:text-green-400' : 'text-muted-foreground'}`}>
                                            {storageInfo.portable_available
                                                ? `✓ ${t('settings.storage.portableAvailable')}`
                                                : t('settings.storage.portableUnavailable')}
                                        </div>
                                    </div>
                                </div>

                                {storageInfo.custom_dir && (
                                    <div className="pt-2 border-t">
                                        <div className="text-xs uppercase tracking-wide text-muted-foreground">
                                            {t('settings.storage.source.custom')}
                                        </div>
                                        <div className="font-mono text-xs break-all">{storageInfo.custom_dir}</div>
                                    </div>
                                )}
                            </div>
                        )}

                        <div className="flex flex-wrap items-center gap-3">
                            <Button variant="outline" onClick={handleOpenStorageDir} disabled={!storageInfo}>
                                <ExternalLink className="mr-2 h-4 w-4" />
                                {t('settings.storage.openActive')}
                            </Button>
                            <Button variant="outline" onClick={handlePickCustomDir} disabled={storageBusy}>
                                <FolderCog className="mr-2 h-4 w-4" />
                                {t('settings.storage.setCustom')}
                            </Button>
                            {storageInfo?.custom_dir && (
                                <Button variant="ghost" onClick={handleClearCustomDir} disabled={storageBusy}>
                                    <RotateCcw className="mr-2 h-4 w-4" />
                                    {t('settings.storage.clearCustom')}
                                </Button>
                            )}
                        </div>
                        <p className="text-xs text-muted-foreground">{t('settings.storage.restartHint')}</p>

                        {storageMessage && (
                            <div className={`rounded-md border p-3 text-sm flex items-start gap-2 ${
                                storageMessage.kind === 'success'
                                    ? 'border-green-500/30 bg-green-500/5 text-green-700 dark:text-green-300'
                                    : 'border-destructive/30 bg-destructive/5 text-destructive'
                            }`}>
                                {storageMessage.kind === 'success'
                                    ? <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0" />
                                    : <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />}
                                <span>{storageMessage.text}</span>
                            </div>
                        )}

                        {storageInfo?.dual_data_detected && !storageInfo.migration_notice_dismissed && (
                            <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-200 flex items-start gap-2">
                                <Info className="mt-0.5 h-4 w-4 shrink-0" />
                                <div className="flex-1 space-y-2">
                                    <p>
                                        {t('settings.storage.noticeMigration', {
                                            portable: storageInfo.portable_dir || '',
                                            default: storageInfo.default_dir,
                                        })}
                                    </p>
                                    <Button size="sm" variant="ghost" onClick={handleDismissMigrationNotice}>
                                        {t('settings.storage.dismiss')}
                                    </Button>
                                </div>
                            </div>
                        )}

                        {storageInfo?.custom_dir && !storageInfo.custom_dir_notice_dismissed && (
                            <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-200 flex items-start gap-2">
                                <Info className="mt-0.5 h-4 w-4 shrink-0" />
                                <div className="flex-1 space-y-2">
                                    <p>
                                        {t('settings.storage.noticeCustom', {
                                            from: storageInfo.portable_dir || storageInfo.default_dir,
                                        })}
                                    </p>
                                    <Button size="sm" variant="ghost" onClick={handleDismissCustomDirNotice}>
                                        {t('settings.storage.dismiss')}
                                    </Button>
                                </div>
                            </div>
                        )}
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
