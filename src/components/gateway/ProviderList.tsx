import { useTranslation } from 'react-i18next';
import { Provider, ProviderStats } from "@/types/gateway";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { GripVertical, Trash2, Plus, Loader2, CheckCircle2, XCircle, Clock, Zap, Coins, Activity } from "lucide-react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from "@dnd-kit/core";
import { arrayMove, SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { Badge } from "@/components/ui/badge";

interface ProviderListProps {
    providers: Provider[];
    providerStatuses: Record<string, string>;
    providerStats: Record<string, ProviderStats>;
    onUpdate: (providers: Provider[]) => void;
    onEdit: (provider: Provider) => void;
    onDelete: (id: string) => void;
    onAdd: () => void;
}

interface SortableProviderItemProps {
    provider: Provider;
    status?: string;
    stats?: ProviderStats;
    onToggle: (id: string, enabled: boolean) => void;
    onEdit: (provider: Provider) => void;
    onDelete: (id: string) => void;
}

function formatNumber(num: number): string {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
    return num.toString();
}

function formatLatency(ms: number): string {
    if (ms >= 1000) return (ms / 1000).toFixed(1) + 's';
    return Math.round(ms) + 'ms';
}

function useTimeAgo() {
    const { t } = useTranslation();
    return (timestamp: number | null): string => {
        if (!timestamp) return t('common.never');
        const now = Date.now() / 1000;
        const diff = now - timestamp;
        if (diff < 60) return t('common.secondsAgo', { count: Math.floor(diff) });
        if (diff < 3600) return t('common.minutesAgo', { count: Math.floor(diff / 60) });
        if (diff < 86400) return t('common.hoursAgo', { count: Math.floor(diff / 3600) });
        return t('common.daysAgo', { count: Math.floor(diff / 86400) });
    };
}

function SortableProviderItem({ provider, status, stats, onToggle, onEdit, onDelete }: SortableProviderItemProps) {
    const { t } = useTranslation();
    const timeAgo = useTimeAgo();
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

    const successRate = stats ? (stats.total_requests > 0 ? (stats.successful_requests / stats.total_requests * 100).toFixed(1) : '100') : '--';
    const isHealthy = stats?.is_healthy ?? true;

    return (
        <div ref={setNodeRef} style={style} className={`p-4 rounded-lg border transition-all ${status === 'pending' ? 'bg-primary/5 border-primary/30 shadow-sm' :
            !isHealthy ? 'bg-destructive/5 border-destructive/20' :
                'bg-muted/30 border-border/50 hover:border-border'
            }`}>
            {/* Header Row */}
            <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-3">
                    <div {...attributes} {...listeners} className="cursor-grab hover:text-foreground text-muted-foreground">
                        <GripVertical className="h-4 w-4" />
                    </div>
                    <div>
                        <div className="flex items-center gap-2">
                            <div className="font-medium">{provider.name}</div>
                            {status === 'pending' && <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />}
                            {status === 'success' && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                            {status === 'error' && <XCircle className="h-3.5 w-3.5 text-destructive" />}
                            {/* API Type Badges */}
                            <div className="flex gap-1">
                                {provider.api_types?.map(type => (
                                    <Badge key={type} variant="secondary" className="text-[10px] px-1.5 py-0">
                                        {type === 'Anthropic' ? 'Claude' : type === 'OpenAIResponses' ? 'CodeX' : 'Chat'}
                                    </Badge>
                                ))}
                            </div>
                        </div>
                        <div className="text-xs text-muted-foreground">{provider.base_url}</div>
                    </div>
                </div>
                <div className="flex items-center gap-2">
                    <Switch
                        checked={provider.enabled}
                        onCheckedChange={(checked) => onToggle(provider.id, checked)}
                    />
                    <Button variant="ghost" size="sm" onClick={() => onEdit(provider)}>
                        {t('common.edit')}
                    </Button>
                    <Button variant="ghost" size="icon" className="h-8 w-8 text-destructive/70 hover:text-destructive" onClick={() => onDelete(provider.id)}>
                        <Trash2 className="h-4 w-4" />
                    </Button>
                </div>
            </div>

            {/* Stats Row */}
            {stats && stats.total_requests > 0 && (
                <div className="grid grid-cols-4 gap-4 pt-3 border-t border-border/30">
                    {/* Requests & Success Rate */}
                    <div className="flex items-center gap-2">
                        <Activity className="h-4 w-4 text-muted-foreground" />
                        <div>
                            <div className="text-sm font-medium">{formatNumber(stats.total_requests)}</div>
                            <div className="text-[10px] text-muted-foreground">
                                {t('gateway.successRate')} <span className={Number(successRate) >= 90 ? 'text-green-500' : Number(successRate) >= 70 ? 'text-yellow-500' : 'text-destructive'}>{successRate}%</span>
                            </div>
                        </div>
                    </div>

                    {/* Latency */}
                    <div className="flex items-center gap-2">
                        <Clock className="h-4 w-4 text-muted-foreground" />
                        <div>
                            <div className="text-sm font-medium">{formatLatency(stats.avg_latency_ms)}</div>
                            <div className="text-[10px] text-muted-foreground">
                                P95: {formatLatency(stats.p95_latency_ms)}
                            </div>
                        </div>
                    </div>

                    {/* Tokens */}
                    <div className="flex items-center gap-2">
                        <Zap className="h-4 w-4 text-muted-foreground" />
                        <div>
                            <div className="text-sm font-medium">{formatNumber(stats.total_input_tokens + stats.total_output_tokens)}</div>
                            <div className="text-[10px] text-muted-foreground">
                                {t('gateway.input')} {formatNumber(stats.total_input_tokens)} / {t('gateway.output')} {formatNumber(stats.total_output_tokens)}
                            </div>
                        </div>
                    </div>

                    {/* Cost */}
                    <div className="flex items-center gap-2">
                        <Coins className="h-4 w-4 text-muted-foreground" />
                        <div>
                            <div className="text-sm font-medium">${stats.total_cost.toFixed(2)}</div>
                            <div className="text-[10px] text-muted-foreground">
                                {isHealthy ? (
                                    <span className="text-green-500">● {t('common.healthy')}</span>
                                ) : (
                                    <span className="text-destructive">● {t('common.cooldown')}</span>
                                )}
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Health Status for unhealthy providers */}
            {stats && !isHealthy && (
                <div className="mt-2 px-2 py-1 rounded bg-destructive/10 text-destructive text-xs">
                    {t('gateway.consecutiveFailures')} {stats.consecutive_failures} · {t('gateway.lastFailure')}: {timeAgo(stats.last_failure_at)} · {stats.last_error_message || t('common.error')}
                </div>
            )}
        </div>
    );
}

export function ProviderList({ providers, providerStatuses, providerStats, onUpdate, onEdit, onDelete, onAdd }: ProviderListProps) {
    const { t } = useTranslation();
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
        <Card className="min-h-[400px]">
            <CardHeader className="flex flex-row items-center justify-between">
                <CardTitle>{t('gateway.providers')}</CardTitle>
                <Button size="sm" onClick={onAdd}>
                    <Plus className="h-4 w-4 mr-2" />
                    {t('gateway.addProvider')}
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
                        <div className="space-y-3">
                            {providers.map((provider) => (
                                <SortableProviderItem
                                    key={provider.id}
                                    provider={provider}
                                    status={providerStatuses[provider.id]}
                                    stats={providerStats[provider.name]}
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
