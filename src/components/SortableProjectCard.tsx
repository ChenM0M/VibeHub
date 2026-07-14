import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ProjectCard } from './ProjectCard';
import { Project } from '@/types';
import { GripVertical } from 'lucide-react';

interface SortableProjectCardProps {
    project: Project;
    onLaunch: (project: Project) => void;
    onCustomLaunch: (project: Project) => void;
    onSelect?: (project: Project) => void;
}

export function SortableProjectCard({ project, onLaunch, onCustomLaunch, onSelect }: SortableProjectCardProps) {
    const {
        attributes,
        listeners,
        setNodeRef,
        transform,
        transition,
        isDragging,
        setActivatorNodeRef,
    } = useSortable({
        id: project.id,
        attributes: {
            role: 'group',
            roleDescription: 'project card',
            tabIndex: -1,
        },
    });

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
        zIndex: isDragging ? 50 : 'auto',
        position: isDragging ? 'relative' as const : undefined,
    };
    return (
        <div
            ref={setNodeRef}
            style={style}
            {...attributes}
            className="relative h-full"
        >
            <ProjectCard
                project={project}
                onLaunch={onLaunch}
                onCustomLaunch={onCustomLaunch}
                onSelect={onSelect}
                dragHandle={
                    <div
                        ref={setActivatorNodeRef}
                        data-project-drag-handle
                        aria-hidden="true"
                        onPointerDown={(event) => listeners?.onPointerDown?.(event)}
                        className="absolute top-3 right-12 z-20 flex h-8 w-8 cursor-grab touch-none items-center justify-center rounded-md text-muted-foreground/50 hover:bg-background/40 hover:text-foreground active:cursor-grabbing"
                    >
                        <GripVertical className="h-4 w-4" />
                    </div>
                }
            />
        </div>
    );
}
