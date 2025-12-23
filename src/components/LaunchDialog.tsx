import { useState, useEffect } from 'react';
import { Project, TagConfig } from '@/types';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useAppStore } from '@/stores/appStore';
import { Play } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';

interface LaunchDialogProps {
    isOpen: boolean;
    onClose: () => void;
    project: Project | null;
    isCustomLaunch?: boolean;
}

export function LaunchDialog({ isOpen, onClose, project, isCustomLaunch = false }: LaunchDialogProps) {
    const { t } = useTranslation();
    const { launchCustom, config } = useAppStore();
    const [customConfig, setCustomConfig] = useState<TagConfig>({
        executable: '',
        args: [],
        env: {}
    });
    const [argsString, setArgsString] = useState('');
    const [showCustomForm, setShowCustomForm] = useState(false);
    const [selectedTagIds, setSelectedTagIds] = useState<string[]>([]);
    const [availableTags, setAvailableTags] = useState<{ id: string, name: string }[]>([]);

    useEffect(() => {
        if (project) {
            // 自定义启动模式或无标签项目都显示所有全局标签，普通模式只显示项目关联的标签
            const shouldShowAllTags = isCustomLaunch || project.tags.length === 0;
            const tagsToShow = shouldShowAllTags
                ? (config?.tags || [])
                : (config?.tags.filter(t => project.tags.includes(t.id)) || []);
            setAvailableTags(tagsToShow);

            // 自定义启动模式或无标签项目默认不选中任何标签，普通模式选中所有项目标签
            setSelectedTagIds(shouldShowAllTags ? [] : tagsToShow.map(t => t.id));

            // 只有当全局标签列表也为空时，才自动显示自定义配置表单
            if (tagsToShow.length === 0) {
                setShowCustomForm(true);

                // Pre-fill based on project type
                if (project.project_type === 'node') {
                    setCustomConfig(prev => ({ ...prev, executable: 'npm', args: ['start'] }));
                    setArgsString('start');
                } else if (project.project_type === 'rust') {
                    setCustomConfig(prev => ({ ...prev, executable: 'cargo', args: ['run'] }));
                    setArgsString('run');
                } else if (project.project_type === 'python') {
                    setCustomConfig(prev => ({ ...prev, executable: 'python', args: ['main.py'] }));
                    setArgsString('main.py');
                }
            } else {
                setShowCustomForm(false);
            }
        }
    }, [project, config, isCustomLaunch]);


    const handleLaunch = async () => {
        if (!project) return;

        if (showCustomForm) {
            // Launch with custom config
            const configToLaunch = {
                ...customConfig,
                args: argsString.split(' ').filter(a => a.length > 0)
            };
            await launchCustom(project.id, configToLaunch);
        } else {
            // Launch with selected tags
            // We need a backend command that accepts specific tag IDs to launch
            // Currently launchTool launches ALL tags.
            // We should probably update launchTool to accept a list of tag IDs?
            // Or just launch all for now if the backend doesn't support filtering.
            // User said "click which tag to use... can also multi-select".
            // So we need to filter.
            // Since we can't easily change backend signature without breaking things, 
            // let's use launchCustom for each selected tag? No, that's sequential.
            // We need to update launch_tool in backend to accept optional tag_ids.
            // For now, let's assume launchTool launches all, but we want to filter.
            // If we can't change backend easily right now (we can, we are the dev), let's update backend.
            // But wait, I can just iterate and call launchCustom for each selected tag's config!

            const selectedTags = config?.tags.filter(t => selectedTagIds.includes(t.id));
            if (selectedTags) {
                for (const tag of selectedTags) {
                    if (tag.config) {
                        await launchCustom(project.id, tag.config);
                    }
                }
            }
        }
        onClose();
    };

    const toggleTag = (tagId: string) => {
        setSelectedTagIds(prev =>
            prev.includes(tagId)
                ? prev.filter(id => id !== tagId)
                : [...prev, tagId]
        );
    };

    if (!project) return null;

    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                    <DialogTitle>{isCustomLaunch ? t('project.customLaunch') : t('project.launch')}</DialogTitle>
                    <DialogDescription>
                        {t('common.confirm')} <span className="font-medium text-foreground">{project.name}</span>?
                    </DialogDescription>
                </DialogHeader>

                <div className="py-4 space-y-4">
                    {!showCustomForm && availableTags.length > 0 && (
                        <div className="space-y-3">
                            <Label>{t('project.tags')}</Label>
                            <div className="grid grid-cols-2 gap-2">
                                {availableTags.map(tag => (
                                    <div key={tag.id} className="flex items-center space-x-2 border p-2 rounded-md">
                                        <Checkbox
                                            id={tag.id}
                                            checked={selectedTagIds.includes(tag.id)}
                                            onCheckedChange={() => toggleTag(tag.id)}
                                        />
                                        <label
                                            htmlFor={tag.id}
                                            className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer"
                                        >
                                            {tag.name}
                                        </label>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}

                    {!showCustomForm && availableTags.length === 0 && isCustomLaunch && (
                        <div className="text-center py-4 text-muted-foreground">
                            <p className="text-sm">{t('settings.tags.noTags', '暂无可用标签，请使用自定义配置')}</p>
                        </div>
                    )}

                    {showCustomForm && (
                        <div className="space-y-4 border rounded-md p-4 bg-muted/30">
                            <div className="flex items-center justify-between">
                                <h4 className="text-sm font-medium">{t('tag.config.title')}</h4>
                                {availableTags.length > 0 && (
                                    <Button variant="ghost" size="sm" onClick={() => setShowCustomForm(false)} className="h-6 text-xs">
                                        {t('common.cancel')}
                                    </Button>
                                )}
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="executable">{t('tag.config.executable')}</Label>
                                <Input
                                    id="executable"
                                    value={customConfig.executable}
                                    onChange={e => setCustomConfig({ ...customConfig, executable: e.target.value })}
                                    placeholder="e.g. npm, cargo, python"
                                />
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="args">{t('tag.config.arguments')}</Label>
                                <Input
                                    id="args"
                                    value={argsString}
                                    onChange={e => setArgsString(e.target.value)}
                                    placeholder="e.g. start, run, main.py"
                                />
                            </div>
                        </div>
                    )}

                    {!showCustomForm && (
                        <Button
                            variant="outline"
                            size="sm"
                            className="w-full"
                            onClick={() => setShowCustomForm(true)}
                        >
                            {t('tag.config.title')} (Custom)
                        </Button>
                    )}
                </div>

                <DialogFooter>
                    <Button variant="outline" onClick={onClose}>
                        {t('common.cancel')}
                    </Button>
                    <Button onClick={handleLaunch} className="gap-2" disabled={!showCustomForm && selectedTagIds.length === 0}>
                        <Play className="h-4 w-4" />
                        {t('project.launch')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
