import { useEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { ChevronDown, X } from 'lucide-react';
import {
    DndContext,
    DragEndEvent,
    KeyboardSensor,
    PointerSensor,
    closestCenter,
    useSensor,
    useSensors,
} from '@dnd-kit/core';
import {
    SortableContext,
    arrayMove,
    horizontalListSortingStrategy,
    sortableKeyboardCoordinates,
    useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useAppStore } from '@/stores/appStore';
import { useTabsStore } from '@/stores/tabsStore';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

interface ProjectTab {
    id: string;
    name: string;
    path: string;
    themeColor?: string;
}

interface ProjectTabItemProps {
    tab: ProjectTab;
    active: boolean;
    onActivate: () => void;
    onClose: () => void;
    closeLabel: string;
}

function ProjectTabItem({ tab, active, onActivate, onClose, closeLabel }: ProjectTabItemProps) {
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: tab.id });

    return (
        <div
            ref={setNodeRef}
            style={{ transform: CSS.Translate.toString(transform ? { ...transform, y: 0 } : null), transition }}
            {...attributes}
            {...listeners}
            role="tab"
            aria-selected={active}
            title={tab.path}
            data-project-tab={tab.id}
            onClick={onActivate}
            onAuxClick={(event) => {
                if (event.button !== 1) return;
                event.preventDefault();
                onClose();
            }}
            className={`group relative flex h-8 min-w-[7.5rem] max-w-[13rem] shrink cursor-pointer select-none items-center gap-2 rounded-t-md border border-b-0 px-3 text-sm transition-colors ${
                active
                    ? 'border-border/60 bg-background text-foreground'
                    : 'border-transparent bg-muted/40 text-muted-foreground hover:bg-muted/70'
            } ${isDragging ? 'z-10 opacity-80' : ''}`}
        >
            <span
                className="h-2 w-2 shrink-0 rounded-full"
                style={{ backgroundColor: tab.themeColor || 'hsl(var(--primary))' }}
            />
            <span className="truncate">{tab.name}</span>
            <button
                type="button"
                aria-label={closeLabel}
                onClick={(event) => {
                    event.stopPropagation();
                    onClose();
                }}
                onPointerDown={(event) => event.stopPropagation()}
                className="ml-auto rounded p-0.5 opacity-0 transition-opacity hover:bg-muted focus:opacity-100 group-hover:opacity-100"
            >
                <X className="h-3 w-3" />
            </button>
            {active && (
                <motion.div
                    layoutId="active-project-tab-indicator"
                    className="absolute inset-x-0 bottom-0 h-0.5 bg-primary"
                />
            )}
        </div>
    );
}

export function ProjectTabBar() {
    const { t } = useTranslation();
    const config = useAppStore((state) => state.config);
    const tabIds = useTabsStore((state) => state.tabs);
    const activeTabId = useTabsStore((state) => state.activeTabId);
    const activateTab = useTabsStore((state) => state.activateTab);
    const activateIndex = useTabsStore((state) => state.activateIndex);
    const activateRelative = useTabsStore((state) => state.activateRelative);
    const closeTab = useTabsStore((state) => state.closeTab);
    const closeActiveTab = useTabsStore((state) => state.closeActiveTab);
    const reorderTabs = useTabsStore((state) => state.reorderTabs);
    const diagnostics = useTabsStore((state) => state.diagnostics);
    const hydrating = useTabsStore((state) => state.hydrating);
    const retryHydrate = useTabsStore((state) => state.retryHydrate);
    const deactivateTabs = useTabsStore((state) => state.deactivateTabs);
    const scrollRef = useRef<HTMLDivElement | null>(null);

    const tabs = useMemo<ProjectTab[]>(() => {
        if (!config) return [];
        return tabIds
            .map((id) => config.projects.find((project) => project.id === id))
            .filter((project): project is NonNullable<typeof project> => Boolean(project))
            .map((project) => ({
                id: project.id,
                name: project.name,
                path: project.path,
                themeColor: project.theme_color,
            }));
    }, [config, tabIds]);

    const sensors = useSensors(
        useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
        useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
    );

    useEffect(() => {
        if (tabs.length === 0) return;
        const handleKeyDown = (event: KeyboardEvent) => {
            const accelerator = event.metaKey || event.ctrlKey;
            if (!accelerator) return;
            if (event.key.toLowerCase() === 'w') {
                event.preventDefault();
                closeActiveTab();
                return;
            }
            if (event.key === 'Tab') {
                event.preventDefault();
                activateRelative(event.shiftKey ? -1 : 1);
                return;
            }
            if (event.shiftKey && (event.key === ']' || event.code === 'BracketRight')) {
                event.preventDefault();
                activateRelative(1);
                return;
            }
            if (event.shiftKey && (event.key === '[' || event.code === 'BracketLeft')) {
                event.preventDefault();
                activateRelative(-1);
                return;
            }
            if (!event.shiftKey && /^[1-9]$/.test(event.key)) {
                event.preventDefault();
                activateIndex(Number(event.key) - 1);
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [tabs.length, activateIndex, activateRelative, closeActiveTab]);

    useEffect(() => {
        const container = scrollRef.current;
        if (!activeTabId || !container) return;
        const node = container.querySelector<HTMLElement>(`[data-project-tab="${activeTabId}"]`);
        if (!node) return;
        const overflowLeft = node.offsetLeft - container.scrollLeft;
        const overflowRight = node.offsetLeft + node.offsetWidth - (container.scrollLeft + container.clientWidth);
        if (overflowLeft < 0) {
            container.scrollLeft += overflowLeft;
        } else if (overflowRight > 0) {
            container.scrollLeft += overflowRight;
        }
    }, [activeTabId, tabs.length]);

    const restoreNotice = diagnostics.length > 0 ? (
        <div role="alert" className="flex shrink-0 items-center gap-2 border-b border-amber-500/30 bg-amber-500/10 px-3 py-1.5 text-xs text-amber-900 dark:text-amber-100">
            <span className="min-w-0 flex-1 truncate">
                {t(`tabs.diagnostics.workspace_state.${diagnostics[0].code.replace(/^workspace_state\./, '')}`, { defaultValue: diagnostics[0].message })}
            </span>
            <button type="button" className="shrink-0 underline underline-offset-2 disabled:opacity-50" onClick={() => void retryHydrate()} disabled={hydrating}>
                {hydrating ? t('tabs.restoring') : t('tabs.retryRestore')}
            </button>
            <button type="button" className="shrink-0 underline underline-offset-2" onClick={deactivateTabs}>
                {t('tabs.openProjectList')}
            </button>
        </div>
    ) : null;

    if (tabs.length === 0) return restoreNotice;

    const handleDragEnd = (event: DragEndEvent) => {
        const { active, over } = event;
        if (!over || active.id === over.id) return;
        const oldIndex = tabIds.indexOf(String(active.id));
        const newIndex = tabIds.indexOf(String(over.id));
        if (oldIndex === -1 || newIndex === -1) return;
        reorderTabs(arrayMove(tabIds, oldIndex, newIndex));
    };

    return (
        <>
            {restoreNotice}
            <div className="flex h-9 shrink-0 items-end gap-1 border-b border-border/60 bg-muted/20 px-3">
            <div
                ref={scrollRef}
                role="tablist"
                aria-label={t('tabs.ariaLabel')}
                className="flex h-full min-w-0 flex-1 flex-nowrap items-end gap-1 overflow-x-auto overflow-y-hidden scrollbar-hidden"
            >
                <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                    <SortableContext items={tabIds} strategy={horizontalListSortingStrategy}>
                        {tabs.map((tab) => (
                            <ProjectTabItem
                                key={tab.id}
                                tab={tab}
                                active={tab.id === activeTabId}
                                onActivate={() => activateTab(tab.id)}
                                onClose={() => closeTab(tab.id)}
                                closeLabel={t('tabs.close')}
                            />
                        ))}
                    </SortableContext>
                </DndContext>
            </div>
            <DropdownMenu>
                <DropdownMenuTrigger
                    className="mb-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
                    aria-label={t('tabs.listAll')}
                >
                    <ChevronDown className="h-4 w-4" />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="max-h-80 w-64 overflow-y-auto">
                    {tabs.map((tab) => (
                        <DropdownMenuItem
                            key={tab.id}
                            onSelect={() => activateTab(tab.id)}
                            className={tab.id === activeTabId ? 'font-medium' : undefined}
                        >
                            <span className="truncate">{tab.name}</span>
                        </DropdownMenuItem>
                    ))}
                </DropdownMenuContent>
            </DropdownMenu>
            </div>
        </>
    );
}
