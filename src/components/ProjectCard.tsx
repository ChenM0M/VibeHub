import { useState } from 'react';
import { Project } from '@/types';
import {
    Star,
    GitBranch,
    Play,
    Terminal,
    Code2,
    Settings,
    Box,
    PlayCircle,
    Folder,
    Cpu,
    Globe,
    Coffee,
    FileCode,
    Hash,
    Edit,
    Trash2,
    Copy
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuSub,
    ContextMenuSubContent,
    ContextMenuSubTrigger,
    ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { useAppStore } from '@/stores/appStore';
import { formatDistanceToNow } from 'date-fns';
import { zhCN, enUS, zhTW } from 'date-fns/locale';
import { ProjectEditDialog } from './ProjectEditDialog';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { useTranslation } from 'react-i18next';
import { getTechStackById } from '@/lib/techStackData';
import { isTagLaunchable } from '@/lib/tagLaunch';

interface ProjectCardProps {
    project: Project;
    onLaunch: (project: Project) => void;
    onCustomLaunch: (project: Project) => void;
    onSelect?: (project: Project) => void;
    dragListeners?: Record<string, any>;
    dragAttributes?: Record<string, any>;
    dragRef?: (element: HTMLElement | null) => void;
}

export function ProjectCard({ project, onLaunch, onCustomLaunch, onSelect, dragListeners, dragAttributes, dragRef }: ProjectCardProps) {
    const { t, i18n } = useTranslation();
    const { toggleProjectStar, openInExplorer, openTerminal, config, deleteProject, launchTool, launchCustom } = useAppStore();
    const [isEditing, setIsEditing] = useState(false);

    const formatDate = (dateString: string) => {
        try {
            let locale = enUS;
            if (i18n.language === 'zh' || i18n.language === 'zh-CN') locale = zhCN;
            if (i18n.language === 'zh-TW' || i18n.language === 'zh-HK') locale = zhTW;

            return formatDistanceToNow(new Date(dateString), {
                addSuffix: true,
                locale
            });
        } catch (e) {
            return '';
        }
    };

    const getTagName = (tagId: string) => {
        const tag = config?.tags.find(t => t.id === tagId);
        return tag?.name || tagId;
    };

    const getTagColor = (tagId: string) => {
        const tag = config?.tags.find(t => t.id === tagId);
        return tag?.color || '#808080';
    };

    const getTagIcon = (tagId: string) => {
        const tag = config?.tags.find(t => t.id === tagId);
        switch (tag?.category) {
            case 'cli': return <Terminal className="h-3 w-3" />;
            case 'ide': return <Code2 className="h-3 w-3" />;
            case 'environment': return <Settings className="h-3 w-3" />;
            case 'startup': return <PlayCircle className="h-3 w-3" />;
            case 'workspace': return <Box className="h-3 w-3" />;
            default: return null;
        }
    };

    const getProjectTypeInfo = (type: string) => {
        switch (type.toLowerCase()) {
            case 'node': return { gradient: 'from-green-500/20 to-emerald-500/20', border: 'border-green-500/30', icon: <Globe className="h-6 w-6 text-green-600 dark:text-green-400" />, label: 'Node.js' };
            case 'rust': return { gradient: 'from-orange-500/20 to-red-500/20', border: 'border-orange-500/30', icon: <Settings className="h-6 w-6 text-orange-600 dark:text-orange-400" />, label: 'Rust' };
            case 'python': return { gradient: 'from-blue-500/20 to-cyan-500/20', border: 'border-blue-500/30', icon: <FileCode className="h-6 w-6 text-blue-600 dark:text-blue-400" />, label: 'Python' };
            case 'java': return { gradient: 'from-red-500/20 to-pink-500/20', border: 'border-red-500/30', icon: <Coffee className="h-6 w-6 text-red-600 dark:text-red-400" />, label: 'Java' };
            case 'go': return { gradient: 'from-cyan-500/20 to-blue-500/20', border: 'border-cyan-500/30', icon: <Cpu className="h-6 w-6 text-cyan-600 dark:text-cyan-400" />, label: 'Go' };
            case 'dotnet': return { gradient: 'from-purple-500/20 to-violet-500/20', border: 'border-purple-500/30', icon: <Hash className="h-6 w-6 text-purple-600 dark:text-purple-400" />, label: '.NET' };
            default: return { gradient: 'from-slate-500/20 to-gray-500/20', border: 'border-slate-500/30', icon: <Box className="h-6 w-6 text-slate-600 dark:text-slate-400" />, label: type };
        }
    };

    const typeInfo = getProjectTypeInfo(project.project_type);
    const originalProject = config?.projects.find(p => p.id === project.id) || project;

    const launchableTags = (config?.tags || []).filter(isTagLaunchable);

    const handleQuickLaunchTag = async (tag: typeof launchableTags[number]) => {
        try {
            await launchCustom(project.id, tag.config!, tag.category);
        } catch (error) {
            console.error('Quick launch failed:', error);
        }
    };

    // Override with custom theme color if present
    const customStyle = project.theme_color ? {
        borderColor: project.theme_color,
    } : {};

    const headerStyle = project.theme_color ? {
        background: `linear-gradient(135deg, ${project.theme_color}20, ${project.theme_color}40)`,
    } : {};

    const hasLaunchableTags = project.tags.some(tagId => {
        const tag = config?.tags.find(t => t.id === tagId);
        return tag ? isTagLaunchable(tag) : false;
    });

    const handleLaunchClick = async (e: React.MouseEvent) => {
        e.stopPropagation();
        if (hasLaunchableTags) {
            try {
                await launchTool(project.id);
            } catch (error) {
                console.error('Launch failed:', error);
                onLaunch(project);
            }
        } else {
            onLaunch(project);
        }
    };

    const handleSelectClick = (event: React.MouseEvent<HTMLDivElement>) => {
        event.stopPropagation();
        onSelect?.(originalProject);
    };

    const handleSelectKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
        if (event.key !== 'Enter' && event.key !== ' ') return;
        event.preventDefault();
        event.stopPropagation();
        onSelect?.(originalProject);
    };

    // Get tech stack items to display (max 4, show +N for overflow)
    const techStackItems = project.tech_stack || [];
    const maxTechDisplay = 4;
    const displayedTech = techStackItems.slice(0, maxTechDisplay);
    const remainingTech = techStackItems.length - maxTechDisplay;

    return (
        <ContextMenu>
            <ContextMenuTrigger asChild>
                <div
                    data-project-card
                    role="group"
                    aria-label={`${project.name} 项目卡片`}
                    className={`group relative flex flex-col justify-between min-h-[180px] h-full bg-card hover:bg-accent/5 border rounded-xl transition-all duration-200 hover:shadow-lg hover:-translate-y-1 overflow-hidden`}
                    style={customStyle}
                >
                    {/* Header / Banner Area */}
                    <div
                        ref={dragRef}
                        {...dragListeners}
                        {...dragAttributes}
                        className={`h-20 relative overflow-hidden transition-all duration-200 ${dragListeners ? 'cursor-grab active:cursor-grabbing touch-none' : ''} ${!project.theme_color && !project.cover_image ? `bg-gradient-to-br ${typeInfo.gradient}` : ''}`}
                        style={project.cover_image ? {
                            backgroundImage: `url(${project.cover_image})`,
                            backgroundSize: 'cover',
                            backgroundPosition: 'center'
                        } : headerStyle}
                    >
                        {project.cover_image && <div className="absolute inset-0 bg-black/20 backdrop-blur-[1px]" />}

                        <div className="absolute top-4 left-4 z-10 flex items-center gap-2">
                            <div className="p-1.5 bg-background/60 backdrop-blur-sm rounded-lg shadow-sm">
                                {project.icon ? (
                                    <img src={project.icon} alt={project.name} className="w-7 h-7 object-contain" />
                                ) : (
                                    <div className="w-7 h-7 flex items-center justify-center">
                                        {typeInfo.icon}
                                    </div>
                                )}
                            </div>
                            {/* Tech Stack Badges */}
                            <div className="flex items-center gap-1">
                                {displayedTech.map(techId => {
                                    const tech = getTechStackById(techId);
                                    return tech ? (
                                        <TooltipProvider key={techId}>
                                            <Tooltip>
                                                <TooltipTrigger asChild>
                                                    <div className="p-1 bg-background/60 backdrop-blur-sm rounded shadow-sm">
                                                        <img src={tech.icon} alt={tech.name} className="w-4 h-4" />
                                                    </div>
                                                </TooltipTrigger>
                                                <TooltipContent side="bottom" className="text-xs">
                                                    {tech.name}
                                                </TooltipContent>
                                            </Tooltip>
                                        </TooltipProvider>
                                    ) : (
                                        <span key={techId} className="px-1.5 py-0.5 bg-background/60 backdrop-blur-sm rounded text-[10px] font-medium">
                                            {techId}
                                        </span>
                                    );
                                })}
                                {remainingTech > 0 && (
                                    <TooltipProvider>
                                        <Tooltip>
                                            <TooltipTrigger asChild>
                                                <span className="px-1.5 py-0.5 bg-background/60 backdrop-blur-sm rounded text-[10px] font-medium">
                                                    +{remainingTech}
                                                </span>
                                            </TooltipTrigger>
                                            <TooltipContent side="bottom" className="text-xs max-w-[200px]">
                                                {techStackItems.slice(maxTechDisplay).map(id => getTechStackById(id)?.name || id).join(', ')}
                                            </TooltipContent>
                                        </Tooltip>
                                    </TooltipProvider>
                                )}
                            </div>
                        </div>

                    </div>

                    {/* Content Area */}
                    <div
                        className="p-4 flex flex-col flex-1 justify-between cursor-pointer group/content"
                        onClick={handleSelectClick}
                        onKeyDown={handleSelectKeyDown}
                        data-project-open
                        role="button"
                        tabIndex={0}
                        aria-label={`打开项目 ${project.name}`}
                    >
                        <TooltipProvider>
                            <Tooltip>
                                <TooltipTrigger asChild>
                                    <div
                                        className="block w-full text-left rounded-sm"
                                    >
                                        <span className="block font-semibold text-base tracking-tight truncate group-hover/content:text-primary transition-colors">
                                            {project.name}
                                        </span>
                                        <span className="block text-xs text-muted-foreground break-all line-clamp-1 group-hover/content:text-foreground transition-colors mt-0.5">
                                            {project.path}
                                        </span>
                                    </div>
                                </TooltipTrigger>
                                <TooltipContent side="bottom" className="max-w-[300px] break-all">
                                    {project.path}
                                </TooltipContent>
                            </Tooltip>
                        </TooltipProvider>

                        <div className="space-y-3 mt-2">
                            <div className="flex flex-wrap gap-1.5 h-[22px] overflow-hidden">
                                {project.tags.map(tagId => (
                                    <div
                                        key={tagId}
                                        className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md text-[10px] font-medium bg-secondary/50 text-secondary-foreground border border-transparent hover:border-border transition-colors"
                                        style={{ borderLeftColor: getTagColor(tagId), borderLeftWidth: '2px' }}
                                    >
                                        {getTagIcon(tagId)}
                                        {getTagName(tagId)}
                                    </div>
                                ))}
                            </div>

                            <div className="flex items-center justify-between pt-2 border-t border-border/50">
                                <div className="flex items-center gap-2 text-xs text-muted-foreground min-w-0 flex-1">
                                    {project.metadata.git_branch && (
                                        <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-secondary/30 min-w-0">
                                            <GitBranch className="h-3 w-3 shrink-0" />
                                            <span className="truncate">{project.metadata.git_branch}</span>
                                        </div>
                                    )}
                                    {project.last_opened && (
                                        <span className="opacity-70 text-[10px] shrink-0 whitespace-nowrap">{formatDate(project.last_opened)}</span>
                                    )}
                                </div>

                                <Button
                                    size="sm"
                                    className="h-7 text-xs opacity-0 group-hover:opacity-100 transition-all shadow-sm hover:shadow-md bg-primary/90 hover:bg-primary text-primary-foreground border-0 px-3 shrink-0 ml-2"
                                    onClick={handleLaunchClick}
                                >
                                    <Play className="h-3 w-3 mr-1.5 shrink-0" />
                                    <span className="whitespace-nowrap">{t('project.launch')}</span>
                                </Button>
                            </div>
                        </div>
                        </div>

                    <div className="absolute top-3 right-3 z-10">
                        <Button
                            variant="ghost"
                            size="icon"
                            aria-label={project.starred ? t('project.unstar') : t('project.star')}
                            className={`h-8 w-8 hover:bg-background/40 ${project.starred ? 'text-yellow-500' : 'text-muted-foreground/50 hover:text-yellow-500'}`}
                            onClick={(e) => { e.stopPropagation(); toggleProjectStar(project.id); }}
                        >
                            <Star className={`h-4 w-4 ${project.starred ? 'fill-current' : ''}`} />
                        </Button>
                    </div>

                    <ProjectEditDialog
                        isOpen={isEditing}
                        onClose={() => setIsEditing(false)}
                        project={project}
                    />
                </div>
            </ContextMenuTrigger>
            <ContextMenuContent className="w-48">
                <ContextMenuItem onClick={() => onLaunch(project)}>
                    <Play className="mr-2 h-4 w-4" />
                    {t('project.launch')}
                </ContextMenuItem>
                {launchableTags.length > 0 ? (
                    <ContextMenuSub>
                        <ContextMenuSubTrigger>
                            <Settings className="mr-2 h-4 w-4" />
                            {t('project.customLaunch')}
                        </ContextMenuSubTrigger>
                        <ContextMenuSubContent className="w-48 max-h-64 overflow-y-auto">
                            {launchableTags.map(tag => (
                                <ContextMenuItem
                                    key={tag.id}
                                    onClick={() => handleQuickLaunchTag(tag)}
                                >
                                    <span
                                        className="mr-2 h-3 w-3 rounded-full inline-block flex-shrink-0"
                                        style={{ backgroundColor: tag.color }}
                                    />
                                    {tag.name}
                                </ContextMenuItem>
                            ))}
                            <ContextMenuSeparator />
                            <ContextMenuItem onClick={() => onCustomLaunch(project)}>
                                <Settings className="mr-2 h-4 w-4" />
                                {t('project.customConfig')}
                            </ContextMenuItem>
                        </ContextMenuSubContent>
                    </ContextMenuSub>
                ) : (
                    <ContextMenuItem onClick={() => onCustomLaunch(project)}>
                        <Settings className="mr-2 h-4 w-4" />
                        {t('project.customLaunch')}
                    </ContextMenuItem>
                )}
                <ContextMenuSeparator />
                <ContextMenuItem onClick={() => {
                    const originalPath = config?.projects.find(p => p.id === project.id)?.path || project.path;
                    openInExplorer(originalPath);
                }}>
                    <Folder className="mr-2 h-4 w-4" />
                    {t('project.openInExplorer')}
                </ContextMenuItem>
                <ContextMenuItem onClick={() => {
                    const originalPath = config?.projects.find(p => p.id === project.id)?.path || project.path;
                    openTerminal(originalPath);
                }}>
                    <Terminal className="mr-2 h-4 w-4" />
                    {t('project.openInTerminal')}
                </ContextMenuItem>
                <ContextMenuItem onClick={() => {
                    const originalPath = config?.projects.find(p => p.id === project.id)?.path || project.path;
                    navigator.clipboard.writeText(originalPath);
                }}>
                    <Copy className="mr-2 h-4 w-4" />
                    {t('project.copyPath')}
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onClick={() => toggleProjectStar(project.id)}>
                    <Star className={`mr-2 h-4 w-4 ${project.starred ? 'fill-yellow-500 text-yellow-500' : ''}`} />
                    {project.starred ? t('project.unstar') : t('project.star')}
                </ContextMenuItem>
                <ContextMenuItem onClick={() => setIsEditing(true)}>
                    <Edit className="mr-2 h-4 w-4" />
                    {t('project.edit')}
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onClick={() => deleteProject(project.id)} className="text-red-600 focus:text-red-600">
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t('common.delete')}
                </ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    );
}
