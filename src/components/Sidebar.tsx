import { useState } from 'react';
import { useAppStore } from '@/stores/appStore';
import {
    Folder,
    Plus,
    Settings,
    LayoutGrid,
    ChevronRight,
    ChevronDown,
    Activity,
    Bot,
    RefreshCw
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from './ui/button';
import { useTranslation } from 'react-i18next';
import { TagEditDialog } from './TagEditDialog';
import { Tag } from '@/types';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { useEffect } from 'react';

type PageType = 'home' | 'settings' | 'gateway' | 'agent-profiles' | 'about';

interface SidebarProps {
    className?: string;
    onNavigate: (page: PageType) => void;
    currentPage: PageType;
    onCheckUpdate?: () => void;
    isCheckingUpdate?: boolean;
}

export function Sidebar({ className, onNavigate, currentPage, onCheckUpdate, isCheckingUpdate }: SidebarProps) {
    const { t } = useTranslation();
    const { config, refreshConfig, selectedWorkspaceId, setSelectedWorkspaceId } = useAppStore();
    const [expandedWorkspaces, setExpandedWorkspaces] = useState<boolean>(true);
    const [expandedTags, setExpandedTags] = useState<boolean>(true);
    const [isMac] = useState(() => /\bMacintosh\b|\bMac OS X\b/.test(navigator.userAgent));
    const [editingTag, setEditingTag] = useState<Tag | null>(null);
    const [isTagDialogOpen, setIsTagDialogOpen] = useState(false);
    const [appVersion, setAppVersion] = useState<string>('');

    useEffect(() => {
        getVersion().then(setAppVersion).catch(() => setAppVersion('1.3.0'));
    }, []);

    const workspaces = config?.workspaces || [];
    const tags = config?.tags || [];
    const brand = (
        <button
            type="button"
            className="mb-5 flex h-10 w-full items-center gap-2.5 rounded-lg px-1 text-left text-lg font-semibold transition-colors hover:bg-accent/50"
            onClick={() => {
                setSelectedWorkspaceId(null);
                onNavigate('home');
            }}
            title="返回主页"
        >
            <span className="grid h-8 w-8 shrink-0 place-items-center overflow-hidden rounded-[0.6rem] bg-white shadow-sm ring-1 ring-black/10 dark:bg-transparent dark:ring-white/5">
                <img src="/app-icon.png" alt="VibeHub" className="h-8 w-8 -translate-y-px scale-[1.05] object-cover dark:hidden" />
                <img src="/logo-dark.jpg" alt="VibeHub Dark" className="h-8 w-8 -translate-y-px scale-[1.05] object-cover hidden dark:block" />
            </span>
            <span className="bg-gradient-to-r from-primary to-primary/70 bg-clip-text text-transparent">VibeHub</span>
        </button>
    );

    const openCreateTagDialog = () => {
        setEditingTag(null);
        setIsTagDialogOpen(true);
    };

    const openEditTagDialog = (tag: Tag) => {
        setEditingTag(tag);
        setIsTagDialogOpen(true);
    };

    const handleSaveTag = async (tag: Tag) => {
        try {
            await invoke(editingTag ? 'update_tag' : 'add_tag', { tag });
            await refreshConfig();
        } catch (error) {
            console.error('Failed to save tag:', error);
        }
    };

    return (
        <div className={cn("w-64 glass border-r border-border/30 h-full flex flex-col", className)}>
            {isMac && <div className="h-12 shrink-0" data-tauri-drag-region="deep" />}

            <div className="p-4">
                {brand}
                <div className="space-y-1">
                    <Button
                        variant={currentPage === 'home' && !selectedWorkspaceId ? "secondary" : "ghost"}
                        className="w-full justify-start rounded-lg"
                        onClick={() => {
                            setSelectedWorkspaceId(null);
                            onNavigate('home');
                        }}
                    >
                        <LayoutGrid className="mr-2 h-4 w-4" />
                        {t('common.workspaces')}
                    </Button>
                    <Button
                        variant={currentPage === 'gateway' ? "secondary" : "ghost"}
                        className="w-full justify-start rounded-lg"
                        onClick={() => onNavigate('gateway')}
                    >
                        <Activity className="mr-2 h-4 w-4" />
                        {t('gateway.title', 'AI Gateway')}
                    </Button>
                    <Button
                        variant={currentPage === 'agent-profiles' ? "secondary" : "ghost"}
                        className="w-full justify-start rounded-lg"
                        onClick={() => onNavigate('agent-profiles')}
                    >
                        <Bot className="mr-2 h-4 w-4" />
                        {t('agentProfiles.navLabel')}
                    </Button>
                    <Button
                        variant={currentPage === 'settings' ? "secondary" : "ghost"}
                        className="w-full justify-start rounded-lg"
                        onClick={() => onNavigate('settings')}
                    >
                        <Settings className="mr-2 h-4 w-4" />
                        {t('common.settings')}
                    </Button>
                </div>
            </div>

            <div className="flex-1 overflow-y-auto px-4 py-2 scrollbar-auto-hide">
                {/* Workspaces Section */}
                <div className="mb-6">
                    <div
                        className="flex items-center justify-between text-sm font-medium text-muted-foreground mb-2 cursor-pointer hover:text-foreground transition-colors"
                        onClick={() => setExpandedWorkspaces(!expandedWorkspaces)}
                    >
                        <div className="flex items-center">
                            {expandedWorkspaces ? <ChevronDown className="mr-1 h-3 w-3" /> : <ChevronRight className="mr-1 h-3 w-3" />}
                            {t('common.workspaces')}
                        </div>
                        <Button variant="ghost" size="icon" className="h-4 w-4 ml-auto" onClick={(e) => {
                            e.stopPropagation();
                            onNavigate('settings');
                        }}>
                            <Plus className="h-3 w-3" />
                        </Button>
                    </div>

                    {expandedWorkspaces && (
                        <div className="space-y-1 ml-2">
                            {workspaces.length === 0 ? (
                                <div className="text-xs text-muted-foreground italic px-2 py-1">{t('settings.workspaces.noWorkspaces')}</div>
                            ) : (
                                workspaces.map(ws => (
                                    <div
                                        key={ws.id}
                                        className={cn(
                                            "group flex items-center justify-between px-2 py-1.5 text-sm rounded-md hover:bg-accent/50 cursor-pointer transition-colors",
                                            selectedWorkspaceId === ws.id && "bg-accent text-accent-foreground font-medium"
                                        )}
                                        onClick={() => {
                                            setSelectedWorkspaceId(
                                                selectedWorkspaceId === ws.id ? null : ws.id
                                            );
                                            onNavigate('home');
                                        }}
                                    >
                                        <div className="flex items-center overflow-hidden">
                                            <Folder className={cn(
                                                "mr-2 h-3.5 w-3.5 flex-shrink-0",
                                                selectedWorkspaceId === ws.id ? "text-primary" : "text-muted-foreground"
                                            )} />
                                            <span className="truncate">{ws.name}</span>
                                        </div>
                                    </div>
                                ))
                            )}
                        </div>
                    )}
                </div>

                {/* Tags Section */}
                <div>
                    <div
                        className="flex items-center justify-between text-sm font-medium text-muted-foreground mb-2 cursor-pointer hover:text-foreground transition-colors"
                        onClick={() => setExpandedTags(!expandedTags)}
                    >
                        <div className="flex items-center">
                            {expandedTags ? <ChevronDown className="mr-1 h-3 w-3" /> : <ChevronRight className="mr-1 h-3 w-3" />}
                            {t('common.tags')}
                        </div>
                        <Button
                            variant="ghost"
                            size="icon"
                            className="h-4 w-4 ml-auto"
                            title={t('tag.createTitle')}
                            aria-label={t('tag.createTitle')}
                            onClick={(e) => {
                                e.stopPropagation();
                                openCreateTagDialog();
                            }}
                        >
                            <Plus className="h-3 w-3" />
                        </Button>
                    </div>

                    {expandedTags && (
                        <div className="space-y-1 ml-2">
                            {tags.length === 0 ? (
                                <div className="text-xs text-muted-foreground italic px-2 py-1">No tags</div>
                            ) : (
                                tags.map(tag => (
                                    <div
                                        key={tag.id}
                                        className="flex items-center px-2 py-1.5 text-sm rounded-md hover:bg-accent/50 cursor-pointer transition-colors"
                                        onClick={() => openEditTagDialog(tag)}
                                    >
                                        <div className="w-2 h-2 rounded-full mr-2" style={{ backgroundColor: tag.color, boxShadow: `0 0 8px ${tag.color}50` }} />
                                        <span className="truncate">{tag.name}</span>
                                    </div>
                                ))
                            )}
                        </div>
                    )}
                </div>
            </div>

            <div className="p-4 border-t border-border/50 backdrop-blur-sm">
                <div className="flex items-center justify-between">
                    <button
                        className="text-xs text-muted-foreground hover:text-primary transition-colors cursor-pointer flex items-center gap-1.5 py-1 px-2 rounded hover:bg-accent/50"
                        onClick={() => onNavigate('about')}
                        title={t('about.title', '关于')}
                    >
                        <span>v{appVersion || '1.3.0'} Portable</span>
                    </button>
                    <button
                        className="text-xs text-muted-foreground hover:text-primary transition-colors cursor-pointer p-1.5 rounded hover:bg-accent/50"
                        onClick={onCheckUpdate}
                        disabled={isCheckingUpdate}
                        title={t('update.checkUpdate', '检查更新')}
                    >
                        <RefreshCw className={cn("h-3.5 w-3.5", isCheckingUpdate && "animate-spin")} />
                    </button>
                </div>
            </div>

            <TagEditDialog
                tag={editingTag || undefined}
                open={isTagDialogOpen}
                onOpenChange={(open) => {
                    setIsTagDialogOpen(open);
                    if (!open) setEditingTag(null);
                }}
                onSave={handleSaveTag}
            />
        </div>
    );
}
