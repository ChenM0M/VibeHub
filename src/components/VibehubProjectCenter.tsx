import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Project } from '@/types';
import { Button } from '@/components/ui/button';
import { VibehubCockpitContent } from './VibehubCockpitDialog';

interface VibehubProjectCenterProps {
    project: Project;
    onBack: () => void;
}

export function VibehubProjectCenter({ project, onBack }: VibehubProjectCenterProps) {
    const { t } = useTranslation();

    return (
        <div className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-7xl flex-col gap-4 p-8">
            <Button variant="ghost" size="sm" className="w-fit -ml-2" onClick={onBack}>
                <ArrowLeft className="mr-2 h-4 w-4" />
                {t('home.title')}
            </Button>
            <VibehubCockpitContent project={project} showOverview />
        </div>
    );
}
