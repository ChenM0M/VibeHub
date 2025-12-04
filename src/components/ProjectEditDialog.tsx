import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Save, Search, Plus, Check } from 'lucide-react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Project } from '@/types';
import { useAppStore } from '@/stores/appStore';
import { TECH_STACK_PRESETS, ICON_PRESETS, CATEGORY_LABELS, getTechStackById } from '@/lib/techStackData';

interface ProjectEditDialogProps {
    isOpen: boolean;
    onClose: () => void;
    project: Project;
}

export function ProjectEditDialog({ isOpen, onClose, project }: ProjectEditDialogProps) {
    const { t } = useTranslation();
    const { config, updateProject } = useAppStore();
    const [name, setName] = useState(project.name);
    const [description, setDescription] = useState(project.description || '');
    const [selectedTags, setSelectedTags] = useState<string[]>(project.tags || []);
    const [icon, setIcon] = useState(project.icon || '');
    const [coverImage] = useState(project.cover_image || '');
    const [themeColor, setThemeColor] = useState(project.theme_color || '');
    const [techStack, setTechStack] = useState<string[]>(project.tech_stack || []);
    const [iconSearch, setIconSearch] = useState('');
    const [techSearch, setTechSearch] = useState('');
    const [customTech, setCustomTech] = useState('');
    const [activeIconTab, setActiveIconTab] = useState<'preset' | 'url'>('preset');

    if (!config) return null;

    // Filter icons based on search
    const filteredIcons = useMemo(() => {
        if (!iconSearch) return ICON_PRESETS;
        const query = iconSearch.toLowerCase();
        return ICON_PRESETS.filter(item =>
            item.name.toLowerCase().includes(query) || item.id.toLowerCase().includes(query)
        );
    }, [iconSearch]);

    // Group tech stack by category
    const groupedTechStack = useMemo(() => {
        const groups: Record<string, typeof TECH_STACK_PRESETS> = {};
        const query = techSearch.toLowerCase();

        TECH_STACK_PRESETS.forEach(item => {
            if (techSearch && !item.name.toLowerCase().includes(query) && !item.id.toLowerCase().includes(query)) {
                return;
            }
            if (!groups[item.category]) {
                groups[item.category] = [];
            }
            groups[item.category].push(item);
        });
        return groups;
    }, [techSearch]);

    const handleSave = async () => {
        await updateProject({
            ...project,
            name,
            description,
            tags: selectedTags,
            icon: icon || undefined,
            cover_image: coverImage || undefined,
            theme_color: themeColor || undefined,
            tech_stack: techStack,
        });
        onClose();
    };

    const toggleTag = (tagId: string) => {
        if (selectedTags.includes(tagId)) {
            setSelectedTags(selectedTags.filter(id => id !== tagId));
        } else {
            setSelectedTags([...selectedTags, tagId]);
        }
    };

    const toggleTechStack = (techId: string) => {
        if (techStack.includes(techId)) {
            setTechStack(techStack.filter(id => id !== techId));
        } else {
            setTechStack([...techStack, techId]);
        }
    };

    const addCustomTech = () => {
        if (customTech.trim() && !techStack.includes(customTech.trim())) {
            setTechStack([...techStack, customTech.trim()]);
            setCustomTech('');
        }
    };

    const selectIconPreset = (iconUrl: string) => {
        setIcon(iconUrl);
    };

    return (
        <Dialog.Root open={isOpen} onOpenChange={onClose}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 animate-in fade-in" />
                <Dialog.Content className="fixed left-[50%] top-[50%] z-50 grid w-full max-w-2xl translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 sm:rounded-lg animate-in fade-in-90 zoom-in-95 max-h-[90vh] overflow-y-auto">
                    <div className="flex flex-col space-y-1.5">
                        <Dialog.Title className="text-lg font-semibold">{t('project.edit')}</Dialog.Title>
                    </div>

                    <div className="grid gap-4 py-4">
                        {/* Basic Info */}
                        <div className="grid grid-cols-2 gap-4">
                            <div className="grid gap-2">
                                <label htmlFor="name" className="text-sm font-medium">{t('tag.name')}</label>
                                <Input id="name" value={name} onChange={(e) => setName(e.target.value)} />
                            </div>
                            <div className="grid gap-2">
                                <label htmlFor="desc" className="text-sm font-medium">{t('project.description') || '描述'}</label>
                                <Input id="desc" value={description} onChange={(e) => setDescription(e.target.value)} />
                            </div>
                        </div>

                        {/* Icon Selector */}
                        <div className="grid gap-2">
                            <label className="text-sm font-medium">{t('project.icon') || '图标'}</label>
                            <div className="flex gap-2 mb-2">
                                <Button
                                    variant={activeIconTab === 'preset' ? 'default' : 'outline'}
                                    size="sm"
                                    onClick={() => setActiveIconTab('preset')}
                                >
                                    {t('project.presetIcon') || '预设图标'}
                                </Button>
                                <Button
                                    variant={activeIconTab === 'url' ? 'default' : 'outline'}
                                    size="sm"
                                    onClick={() => setActiveIconTab('url')}
                                >
                                    {t('project.customUrl') || '自定义 URL'}
                                </Button>
                            </div>

                            {activeIconTab === 'preset' ? (
                                <div className="space-y-2">
                                    <div className="relative">
                                        <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                        <Input
                                            value={iconSearch}
                                            onChange={(e) => setIconSearch(e.target.value)}
                                            placeholder={t('project.searchIcons') || '搜索图标...'}
                                            className="pl-8"
                                        />
                                    </div>
                                    <div className="grid grid-cols-8 gap-2 max-h-32 overflow-y-auto p-2 border rounded-md bg-muted/20">
                                        {filteredIcons.map(item => (
                                            <button
                                                key={item.id}
                                                type="button"
                                                onClick={() => selectIconPreset(item.icon)}
                                                className={`p-2 rounded-lg border transition-all hover:scale-105 ${icon === item.icon
                                                    ? 'border-primary bg-primary/10 ring-2 ring-primary/30'
                                                    : 'border-transparent hover:border-border hover:bg-muted/50'
                                                    }`}
                                                title={item.name}
                                            >
                                                <img src={item.icon} alt={item.name} className="w-6 h-6" />
                                            </button>
                                        ))}
                                    </div>
                                </div>
                            ) : (
                                <div className="flex gap-2 items-center">
                                    {icon && (
                                        <div className="p-2 border rounded-lg bg-muted/20">
                                            <img src={icon} alt="icon" className="w-8 h-8" onError={(e) => (e.currentTarget.style.display = 'none')} />
                                        </div>
                                    )}
                                    <Input
                                        value={icon}
                                        onChange={(e) => setIcon(e.target.value)}
                                        placeholder="https://example.com/icon.png"
                                        className="flex-1"
                                    />
                                </div>
                            )}
                        </div>

                        {/* Tech Stack Selector */}
                        <div className="grid gap-2">
                            <label className="text-sm font-medium">{t('project.techStack') || '技术栈'}</label>

                            {/* Selected Tech Stack */}
                            {techStack.length > 0 && (
                                <div className="flex flex-wrap gap-1.5 p-2 border rounded-md bg-muted/20 min-h-[36px]">
                                    {techStack.map(id => {
                                        const preset = getTechStackById(id);
                                        return (
                                            <span
                                                key={id}
                                                className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-background border cursor-pointer hover:bg-destructive/10 hover:border-destructive/30 transition-colors"
                                                onClick={() => toggleTechStack(id)}
                                                style={{ borderColor: preset?.color }}
                                            >
                                                {preset?.icon && <img src={preset.icon} alt="" className="w-3.5 h-3.5" />}
                                                {preset?.name || id}
                                                <X className="h-3 w-3 ml-0.5" />
                                            </span>
                                        );
                                    })}
                                </div>
                            )}

                            {/* Search */}
                            <div className="relative">
                                <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                <Input
                                    value={techSearch}
                                    onChange={(e) => setTechSearch(e.target.value)}
                                    placeholder={t('project.searchTech') || '搜索技术栈...'}
                                    className="pl-8"
                                />
                            </div>

                            {/* Grouped Presets */}
                            <div className="max-h-40 overflow-y-auto border rounded-md p-2 bg-muted/20 space-y-3">
                                {Object.entries(groupedTechStack).map(([category, items]) => (
                                    <div key={category}>
                                        <div className="text-xs font-medium text-muted-foreground mb-1.5">
                                            {CATEGORY_LABELS[category] || category}
                                        </div>
                                        <div className="flex flex-wrap gap-1.5">
                                            {items.map(item => (
                                                <button
                                                    key={item.id}
                                                    type="button"
                                                    onClick={() => toggleTechStack(item.id)}
                                                    className={`inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium transition-all ${techStack.includes(item.id)
                                                        ? 'bg-primary text-primary-foreground'
                                                        : 'bg-background border hover:border-primary/50'
                                                        }`}
                                                >
                                                    <img src={item.icon} alt="" className="w-3.5 h-3.5" />
                                                    {item.name}
                                                    {techStack.includes(item.id) && <Check className="h-3 w-3 ml-0.5" />}
                                                </button>
                                            ))}
                                        </div>
                                    </div>
                                ))}
                            </div>

                            {/* Custom Tech Input */}
                            <div className="flex gap-2">
                                <Input
                                    value={customTech}
                                    onChange={(e) => setCustomTech(e.target.value)}
                                    placeholder={t('project.addCustomTech') || '添加自定义技术栈...'}
                                    onKeyDown={(e) => e.key === 'Enter' && addCustomTech()}
                                    className="flex-1"
                                />
                                <Button type="button" variant="outline" size="icon" onClick={addCustomTech}>
                                    <Plus className="h-4 w-4" />
                                </Button>
                            </div>
                        </div>

                        {/* Theme Color */}
                        <div className="grid gap-2">
                            <label htmlFor="theme" className="text-sm font-medium">{t('project.themeColor') || '主题色'}</label>
                            <div className="flex gap-2">
                                <Input
                                    id="theme"
                                    value={themeColor}
                                    onChange={(e) => setThemeColor(e.target.value)}
                                    placeholder="#RRGGBB"
                                    className="flex-1"
                                />
                                <input
                                    type="color"
                                    value={themeColor || '#000000'}
                                    onChange={(e) => setThemeColor(e.target.value)}
                                    className="h-10 w-10 p-1 rounded border cursor-pointer"
                                />
                            </div>
                        </div>

                        {/* Tags */}
                        <div className="grid gap-2">
                            <label className="text-sm font-medium">{t('project.tags')}</label>
                            <div className="flex flex-wrap gap-2 p-3 border border-input rounded-md min-h-[3rem]">
                                {config.tags.map(tag => (
                                    <button
                                        key={tag.id}
                                        onClick={() => toggleTag(tag.id)}
                                        className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium transition-colors border ${selectedTags.includes(tag.id)
                                            ? 'brightness-95'
                                            : 'opacity-50 hover:opacity-100 bg-transparent'
                                            }`}
                                        style={{
                                            backgroundColor: selectedTags.includes(tag.id) ? tag.color : 'transparent',
                                            borderColor: tag.color,
                                            color: selectedTags.includes(tag.id) ? '#fff' : tag.color
                                        }}
                                    >
                                        {tag.name}
                                    </button>
                                ))}
                            </div>
                        </div>
                    </div>

                    <div className="flex justify-end gap-2">
                        <Button variant="outline" onClick={onClose}>{t('common.cancel')}</Button>
                        <Button onClick={handleSave}>
                            <Save className="mr-2 h-4 w-4" />
                            {t('common.save')}
                        </Button>
                    </div>

                    <Dialog.Close asChild>
                        <Button variant="ghost" className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground">
                            <X className="h-4 w-4" />
                            <span className="sr-only">Close</span>
                        </Button>
                    </Dialog.Close>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
