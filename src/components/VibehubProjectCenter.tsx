import { ArrowLeft, LayoutDashboard } from 'lucide-react';
import { Project } from '@/types';
import { Button } from '@/components/ui/button';
import { VibehubCockpitContent } from './VibehubCockpitDialog';

interface VibehubProjectCenterProps {
    project: Project;
    onBack: () => void;
}

export function VibehubProjectCenter({ project, onBack }: VibehubProjectCenterProps) {
    return (
        <div className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-7xl flex-col gap-6 p-8">
            <div className="flex flex-wrap items-start justify-between gap-4 border-b pb-4">
                <div className="min-w-0">
                    <Button variant="ghost" size="sm" className="-ml-2 mb-3" onClick={onBack}>
                        <ArrowLeft className="mr-2 h-4 w-4" />
                        Projects
                    </Button>
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <LayoutDashboard className="h-4 w-4" />
                        VibeHub v2 Center
                    </div>
                    <h1 className="mt-1 truncate text-2xl font-semibold tracking-tight">{project.name}</h1>
                    <p className="mt-1 break-all text-sm text-muted-foreground">{project.path}</p>
                </div>
            </div>

            <VibehubCockpitContent project={project} showOverview />
        </div>
    );
}
