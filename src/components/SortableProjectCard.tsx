import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ProjectCard } from './ProjectCard';
import { Project } from '@/types';

interface SortableProjectCardProps {
    project: Project;
    onLaunch: (project: Project) => void;
    onCustomLaunch: (project: Project) => void;
}

export function SortableProjectCard({ project, onLaunch, onCustomLaunch }: SortableProjectCardProps) {
    const {
        attributes,
        listeners,
        setNodeRef,
        transform,
        transition,
        isDragging,
    } = useSortable({ id: project.id });

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
        zIndex: isDragging ? 50 : 'auto',
        position: isDragging ? 'relative' as const : undefined,
    };

    return (
        <div ref={setNodeRef} style={style} {...attributes} {...listeners} className="h-full">
            <ProjectCard project={project} onLaunch={onLaunch} onCustomLaunch={onCustomLaunch} />
        </div>
    );
}
