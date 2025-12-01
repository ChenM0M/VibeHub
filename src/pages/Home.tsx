import { useState } from 'react';
import { useAppStore } from '@/stores/appStore';
import { LaunchDialog } from '@/components/LaunchDialog';
import { Project } from '@/types';
import { FolderSearch, RefreshCw, FolderOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuTrigger,
} from "@/components/ui/context-menu"
import {
    DndContext,
    closestCenter,
    KeyboardSensor,
    PointerSensor,
    useSensor,
    useSensors,
    DragEndEvent,
} from '@dnd-kit/core';
import {
    arrayMove,
    SortableContext,
    sortableKeyboardCoordinates,
    rectSortingStrategy,
} from '@dnd-kit/sortable';
import { SortableProjectCard } from '@/components/SortableProjectCard';

interface HomeProps {
    searchQuery: string;
}

export function Home({ searchQuery }: HomeProps) {
    const { t } = useTranslation();
    const { config, reorderProjects, refreshConfig } = useAppStore();
    const [launchProject, setLaunchProject] = useState<Project | null>(null);
    const [isScanning, setIsScanning] = useState(false);
    const [isDragging, setIsDragging] = useState(false);

    const sensors = useSensors(
        useSensor(PointerSensor, {
            activationConstraint: {
                distance: 8,
            },
        }),
        useSensor(KeyboardSensor, {
            coordinateGetter: sortableKeyboardCoordinates,
        })
    );

    const handleDragEnd = (event: DragEndEvent) => {
        const { active, over } = event;

        if (active.id !== over?.id && config) {
            const oldIndex = config.projects.findIndex((p) => p.id === active.id);
            const newIndex = config.projects.findIndex((p) => p.id === over?.id);

            if (oldIndex !== -1 && newIndex !== -1) {
                const newProjects = arrayMove(config.projects, oldIndex, newIndex);
                reorderProjects(newProjects);
            }
        }
    };

    const handleRefresh = async () => {
        setIsScanning(true);
        try {
            await refreshConfig();
        } finally {
            setIsScanning(false);
        }
    };

    const onDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(true);
    };

    const onDragLeave = (e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(false);
    };

    const onDrop = async (e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(false);
        // Drag and drop file logic can be added here
    };

    if (!config) return null;

    const filteredProjects = config.projects.filter((p) =>
        p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        p.path.toLowerCase().includes(searchQuery.toLowerCase()) ||
        p.tags.some(t => t.toLowerCase().includes(searchQuery.toLowerCase()))
    );

    const starredProjects = filteredProjects.filter(p => p.starred);
    const otherProjects = filteredProjects.filter(p => !p.starred);

    return (
        <ContextMenu>
            <ContextMenuTrigger className="flex-1 overflow-y-auto p-8 block h-full relative">
                <div
                    className="max-w-7xl mx-auto space-y-8 min-h-[calc(100vh-4rem)]"
                    onDragOver={onDragOver}
                    onDragLeave={onDragLeave}
                    onDrop={onDrop}
                >
                    {isDragging && (
                        <div className="absolute inset-0 flex items-center justify-center bg-background/80 backdrop-blur-sm z-50 rounded-xl border-2 border-dashed border-primary">
                            <div className="text-center animate-in fade-in zoom-in duration-300">
                                <FolderOpen className="h-16 w-16 mx-auto text-primary mb-4" />
                                <h3 className="text-xl font-semibold">{t('home.dropTitle')}</h3>
                                <p className="text-muted-foreground">{t('home.dropDescription')}</p>
                            </div>
                        </div>
                    )}

                    <div className="flex items-center justify-between">
                        <div>
                            <h1 className="text-3xl font-bold tracking-tight">{t('home.title')}</h1>
                            <p className="text-muted-foreground mt-1">
                                {filteredProjects.length} {t('home.totalProjects')}
                            </p>
                        </div>
                        <Button variant="outline" onClick={handleRefresh} disabled={isScanning}>
                            <RefreshCw className={`mr-2 h-4 w-4 ${isScanning ? 'animate-spin' : ''}`} />
                            {isScanning ? t('home.scanning') : t('home.refresh')}
                        </Button>
                    </div>

                    <DndContext
                        sensors={sensors}
                        collisionDetection={closestCenter}
                        onDragEnd={handleDragEnd}
                    >
                        {starredProjects.length > 0 && (
                            <div className="space-y-4">
                                <h2 className="text-lg font-semibold flex items-center gap-2">
                                    <span className="text-yellow-500">★</span> {t('home.starredProjects')}
                                </h2>
                                <SortableContext
                                    items={starredProjects.map(p => p.id)}
                                    strategy={rectSortingStrategy}
                                >
                                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                                        {starredProjects.map((project) => (
                                            <SortableProjectCard
                                                key={project.id}
                                                project={project}
                                                onLaunch={() => setLaunchProject(project)}
                                            />
                                        ))}
                                    </div>
                                </SortableContext>
                            </div>
                        )}

                        {otherProjects.length > 0 && (
                            <div className="space-y-4">
                                {starredProjects.length > 0 && (
                                    <h2 className="text-lg font-semibold text-muted-foreground">
                                        {t('home.allProjects')}
                                    </h2>
                                )}
                                <SortableContext
                                    items={otherProjects.map(p => p.id)}
                                    strategy={rectSortingStrategy}
                                >
                                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                                        {otherProjects.map((project) => (
                                            <SortableProjectCard
                                                key={project.id}
                                                project={project}
                                                onLaunch={() => setLaunchProject(project)}
                                            />
                                        ))}
                                    </div>
                                </SortableContext>
                            </div>
                        )}
                    </DndContext>

                    {filteredProjects.length === 0 && (
                        <div className="text-center py-20">
                            <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-muted mb-4">
                                <FolderSearch className="h-8 w-8 text-muted-foreground" />
                            </div>
                            <h3 className="text-lg font-medium">{t('home.noProjects')}</h3>
                            <p className="text-muted-foreground mt-2 max-w-sm mx-auto">
                                {searchQuery
                                    ? `No projects matching "${searchQuery}"`
                                    : "Add a workspace in settings to get started."}
                            </p>
                        </div>
                    )}
                </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem onClick={handleRefresh} disabled={isScanning}>
                    <RefreshCw className={`mr-2 h-4 w-4 ${isScanning ? 'animate-spin' : ''}`} />
                    {t('home.refresh')}
                </ContextMenuItem>
            </ContextMenuContent>

            <LaunchDialog
                isOpen={!!launchProject}
                onClose={() => setLaunchProject(null)}
                project={launchProject}
            />
        </ContextMenu>
    );
}
