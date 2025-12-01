import { Provider } from "@/types/gateway";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { GripVertical, Trash2, Plus, Loader2, CheckCircle2, XCircle } from "lucide-react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from "@dnd-kit/core";
import { arrayMove, SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";

interface ProviderListProps {
    providers: Provider[];
    providerStatuses: Record<string, string>;
    onUpdate: (providers: Provider[]) => void;
    onEdit: (provider: Provider) => void;
    onDelete: (id: string) => void;
    onAdd: () => void;
}

interface SortableProviderItemProps {
    provider: Provider;
    status?: string;
    onToggle: (id: string, enabled: boolean) => void;
    onEdit: (provider: Provider) => void;
    onDelete: (id: string) => void;
}

function SortableProviderItem({ provider, status, onToggle, onEdit, onDelete }: SortableProviderItemProps) {
    const {
        attributes,
        listeners,
        setNodeRef,
        transform,
        transition,
    } = useSortable({ id: provider.id });

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
    };

    return (
        <div ref={setNodeRef} style={style} className={`flex items-center justify-between p-3 rounded-lg border border-border/50 transition-colors ${status === 'pending' ? 'bg-primary/5 border-primary/20' : 'bg-muted/50'}`}>
            <div className="flex items-center gap-3">
                <div {...attributes} {...listeners} className="cursor-grab hover:text-foreground text-muted-foreground">
                    <GripVertical className="h-4 w-4" />
                </div>
                <div>
                    <div className="flex items-center gap-2">
                        <div className="font-medium text-sm">{provider.name}</div>
                        {status === 'pending' && <Loader2 className="h-3 w-3 animate-spin text-primary" />}
                        {status === 'success' && <CheckCircle2 className="h-3 w-3 text-green-500" />}
                        {status === 'error' && <XCircle className="h-3 w-3 text-destructive" />}
                    </div>
                    <div className="text-xs text-muted-foreground">{provider.base_url}</div>
                </div>
            </div>
            <div className="flex items-center gap-3">
                <Switch
                    checked={provider.enabled}
                    onCheckedChange={(checked) => onToggle(provider.id, checked)}
                />
                <Button variant="ghost" size="sm" onClick={() => onEdit(provider)}>
                    Edit
                </Button>
                <Button variant="ghost" size="icon" className="h-8 w-8 text-destructive/70 hover:text-destructive" onClick={() => onDelete(provider.id)}>
                    <Trash2 className="h-4 w-4" />
                </Button>
            </div>
        </div>
    );
}

export function ProviderList({ providers, providerStatuses, onUpdate, onEdit, onDelete, onAdd }: ProviderListProps) {
    const sensors = useSensors(
        useSensor(PointerSensor),
        useSensor(KeyboardSensor)
    );

    const handleDragEnd = (event: DragEndEvent) => {
        const { active, over } = event;

        if (active.id !== over?.id) {
            const oldIndex = providers.findIndex((p) => p.id === active.id);
            const newIndex = providers.findIndex((p) => p.id === over?.id);
            onUpdate(arrayMove(providers, oldIndex, newIndex));
        }
    };

    const handleToggle = (id: string, enabled: boolean) => {
        const newProviders = providers.map(p =>
            p.id === id ? { ...p, enabled } : p
        );
        onUpdate(newProviders);
    };

    return (
        <Card className="col-span-3">
            <CardHeader className="flex flex-row items-center justify-between">
                <CardTitle>Providers</CardTitle>
                <Button size="sm" onClick={onAdd}>
                    <Plus className="h-4 w-4 mr-2" />
                    Add Provider
                </Button>
            </CardHeader>
            <CardContent>
                <DndContext
                    sensors={sensors}
                    collisionDetection={closestCenter}
                    onDragEnd={handleDragEnd}
                >
                    <SortableContext
                        items={providers.map(p => p.id)}
                        strategy={verticalListSortingStrategy}
                    >
                        <div className="space-y-2">
                            {providers.map((provider) => (
                                <SortableProviderItem
                                    key={provider.id}
                                    provider={provider}
                                    status={providerStatuses[provider.id]}
                                    onToggle={handleToggle}
                                    onEdit={onEdit}
                                    onDelete={onDelete}
                                />
                            ))}
                        </div>
                    </SortableContext>
                </DndContext>
            </CardContent>
        </Card>
    );
}
