import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Save } from 'lucide-react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Project } from '@/types';
import { useAppStore } from '@/stores/appStore';

interface ProjectEditDialogProps {
    isOpen: boolean;
    onClose: () => void;
    project: Project;
}

export function ProjectEditDialog({ isOpen, onClose, project }: ProjectEditDialogProps) {
    const { config, updateProject } = useAppStore();
    const [name, setName] = useState(project.name);
    const [description, setDescription] = useState(project.description || '');
    const [selectedTags, setSelectedTags] = useState<string[]>(project.tags || []);
    const [icon, setIcon] = useState(project.icon || '');
    const [coverImage, setCoverImage] = useState(project.cover_image || '');
    const [themeColor, setThemeColor] = useState(project.theme_color || '');

    if (!config) return null;

    const handleSave = async () => {
        await updateProject({
            ...project,
            name,
            description,
            tags: selectedTags,
            icon: icon || undefined,
            cover_image: coverImage || undefined,
            theme_color: themeColor || undefined
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

    return (
        <Dialog.Root open={isOpen} onOpenChange={onClose}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 animate-in fade-in" />
                <Dialog.Content className="fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 sm:rounded-lg animate-in fade-in-90 zoom-in-95 max-h-[90vh] overflow-y-auto">
                    <div className="flex flex-col space-y-1.5">
                        <Dialog.Title className="text-lg font-semibold">Edit Project</Dialog.Title>
                    </div>

                    <div className="grid gap-4 py-4">
                        <div className="grid gap-2">
                            <label htmlFor="name" className="text-sm font-medium">Name</label>
                            <Input id="name" value={name} onChange={(e) => setName(e.target.value)} />
                        </div>

                        <div className="grid gap-2">
                            <label htmlFor="desc" className="text-sm font-medium">Description</label>
                            <Input id="desc" value={description} onChange={(e) => setDescription(e.target.value)} />
                        </div>

                        <div className="grid gap-2">
                            <label htmlFor="icon" className="text-sm font-medium">Icon URL (Optional)</label>
                            <Input
                                id="icon"
                                value={icon}
                                onChange={(e) => setIcon(e.target.value)}
                                placeholder="https://example.com/icon.png"
                            />
                        </div>

                        <div className="grid gap-2">
                            <label htmlFor="cover" className="text-sm font-medium">Cover Image URL (Optional)</label>
                            <Input
                                id="cover"
                                value={coverImage}
                                onChange={(e) => setCoverImage(e.target.value)}
                                placeholder="https://example.com/cover.jpg"
                            />
                        </div>

                        <div className="grid gap-2">
                            <label htmlFor="theme" className="text-sm font-medium">Theme Color (Optional)</label>
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

                        <div className="grid gap-2">
                            <label className="text-sm font-medium">Tags</label>
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
                                            color: selectedTags.includes(tag.id) ? '#fff' : tag.color // Simple contrast fix
                                        }}
                                    >
                                        {tag.name}
                                    </button>
                                ))}
                            </div>
                        </div>
                    </div>

                    <div className="flex justify-end gap-2">
                        <Button variant="outline" onClick={onClose}>Cancel</Button>
                        <Button onClick={handleSave}>
                            <Save className="mr-2 h-4 w-4" />
                            Save Changes
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
