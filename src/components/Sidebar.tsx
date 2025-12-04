import { useState } from 'react';
import { useAppStore } from '@/stores/appStore';
import {
    Folder,
    Plus,
    Settings,
    LayoutGrid,
    ChevronRight,
    ChevronDown,
    Activity
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from './ui/button';
import { useTranslation } from 'react-i18next';
import { TagEditDialog } from './TagEditDialog';
import { Tag } from '@/types';
import { invoke } from '@tauri-apps/api/core';

interface SidebarProps {
    className?: string;
    onNavigate: (page: 'home' | 'settings' | 'gateway') => void;
    currentPage: 'home' | 'settings' | 'gateway';
}

export function Sidebar({ className, onNavigate, currentPage }: SidebarProps) {
    const { t } = useTranslation();
    const { config, refreshConfig, selectedWorkspaceId, setSelectedWorkspaceId } = useAppStore();
    const [expandedWorkspaces, setExpandedWorkspaces] = useState<boolean>(true);
    const [expandedTags, setExpandedTags] = useState<boolean>(true);
    const [editingTag, setEditingTag] = useState<Tag | null>(null);

    const workspaces = config?.workspaces || [];
    const tags = config?.tags || [];

    const handleSaveTag = async (tag: Tag) => {
        try {
            await invoke('update_tag', { tag });
            await refreshConfig();
        } catch (error) {
            console.error('Failed to update tag:', error);
        }
    };

    return (
        <div className={cn("w-64 glass border-r border-border/50 h-full flex flex-col", className)}>
            <div className="p-4">
                <div className="flex items-center gap-2 font-semibold text-lg mb-6">
                    <img src="/app-icon.png" alt="VibeHub" className="w-8 h-8 rounded-lg" />
                    <span className="bg-gradient-to-r from-primary to-primary/70 bg-clip-text text-transparent">VibeHub</span>
                </div>

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
                        variant={currentPage === 'settings' ? "secondary" : "ghost"}
                        className="w-full justify-start rounded-lg"
                        onClick={() => onNavigate('settings')}
                    >
                        <Settings className="mr-2 h-4 w-4" />
                        {t('common.settings')}
                    </Button>
                </div>
            </div>

            <div className="flex-1 overflow-y-auto px-4 py-2">
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
                                            if (currentPage !== 'home') onNavigate('home');
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
                        <Button variant="ghost" size="icon" className="h-4 w-4 ml-auto" onClick={(e) => {
                            e.stopPropagation();
                            onNavigate('settings');
                        }}>
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
                                        onClick={() => setEditingTag(tag)}
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
                <div className="text-xs text-muted-foreground text-center">
                    v1.2.1 Portable
                </div>
            </div>

            <TagEditDialog
                tag={editingTag || undefined}
                open={!!editingTag}
                onOpenChange={(open) => !open && setEditingTag(null)}
                onSave={handleSaveTag}
            />
        </div>
    );
}
