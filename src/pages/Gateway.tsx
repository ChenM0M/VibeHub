import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Activity, Server, Coins, Database, Zap } from 'lucide-react';
import { GatewayConfig, Provider, GatewayStats } from '@/types/gateway';
import { ProviderForm } from '@/components/gateway/ProviderForm';
import { StatsCard } from '@/components/gateway/StatsCard';
import { RequestChart } from '@/components/gateway/RequestChart';
import { ProviderList } from '@/components/gateway/ProviderList';
import { formatDistanceToNow } from 'date-fns';

interface ProviderStatusEvent {
    provider_id: string;
    status: string;
}

export function Gateway() {
    const [config, setConfig] = useState<GatewayConfig | null>(null);
    const [stats, setStats] = useState<GatewayStats | null>(null);
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingProvider, setEditingProvider] = useState<Provider | undefined>(undefined);
    const [providerStatuses, setProviderStatuses] = useState<Record<string, string>>({});

    const loadConfig = async () => {
        try {
            const c = await invoke<GatewayConfig>('get_gateway_config');
            setConfig(c);
        } catch (e) {
            console.error('Failed to load config:', e);
        }
    };

    const loadStats = async () => {
        try {
            const s = await invoke<GatewayStats>('get_gateway_stats');
            setStats(s);
        } catch (e) {
            console.error('Failed to load stats:', e);
        }
    };

    useEffect(() => {
        loadConfig();
        const interval = setInterval(loadStats, 5000);
        loadStats();

        // Listen for provider status events
        const unlistenPromise = listen<ProviderStatusEvent>('gateway://provider-status', (event) => {
            setProviderStatuses(prev => ({
                ...prev,
                [event.payload.provider_id]: event.payload.status
            }));

            // Clear status after 3 seconds if success or error
            if (event.payload.status !== 'pending') {
                setTimeout(() => {
                    setProviderStatuses(prev => {
                        const next = { ...prev };
                        delete next[event.payload.provider_id];
                        return next;
                    });
                }, 3000);
                // Also reload stats immediately on completion
                loadStats();
            }
        });

        return () => {
            clearInterval(interval);
            unlistenPromise.then(unlisten => unlisten());
        };
    }, []);

    const handleSaveConfig = async (newConfig: GatewayConfig) => {
        try {
            await invoke('save_gateway_config', { config: newConfig });
            setConfig(newConfig);
        } catch (e) {
            console.error('Failed to save config:', e);
        }
    };

    const handleToggleGateway = (enabled: boolean) => {
        if (!config) return;
        handleSaveConfig({ ...config, enabled });
    };

    const handleProvidersReorder = (providers: Provider[]) => {
        if (!config) return;
        handleSaveConfig({ ...config, providers });
    };

    const handleAddProvider = async (provider: Provider) => {
        if (!config) return;
        const newProviders = [...config.providers, provider];
        await handleSaveConfig({ ...config, providers: newProviders });
        setIsFormOpen(false);
    };

    const handleEditProvider = async (provider: Provider) => {
        if (!config) return;
        const newProviders = config.providers.map(p => p.id === provider.id ? provider : p);
        await handleSaveConfig({ ...config, providers: newProviders });
        setIsFormOpen(false);
        setEditingProvider(undefined);
    };

    const handleDeleteProvider = async (id: string) => {
        if (!config) return;
        const newProviders = config.providers.filter(p => p.id !== id);
        await handleSaveConfig({ ...config, providers: newProviders });
    };

    if (!config) return <div className="p-8">Loading...</div>;

    return (
        <div className="space-y-6 p-8 max-w-[1600px] mx-auto">
            <div className="flex items-center justify-between">
                <div>
                    <h1 className="text-3xl font-bold tracking-tight">AI 网关</h1>
                    <p className="text-muted-foreground mt-2">
                        Manage your local AI proxy and providers
                    </p>
                </div>
                <div className="flex items-center gap-4">
                    <div className="flex items-center gap-2">
                        <span className="text-sm font-medium">Global Switch</span>
                        <Switch checked={config.enabled} onCheckedChange={handleToggleGateway} />
                    </div>
                    <div className={`px-3 py-1 rounded-full text-sm font-medium flex items-center gap-2 ${config.enabled ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'}`}>
                        <Activity className="h-4 w-4" />
                        {config.enabled ? 'Running on :12345' : 'Stopped'}
                    </div>
                </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                <StatsCard
                    title="Total Requests"
                    value={stats?.total_requests.toLocaleString() || '0'}
                    description="Lifetime requests"
                    icon={Server}
                />
                <StatsCard
                    title="Token Usage"
                    value={((stats?.total_input_tokens || 0) + (stats?.total_output_tokens || 0)).toLocaleString()}
                    description={`In: ${(stats?.total_input_tokens || 0).toLocaleString()} / Out: ${(stats?.total_output_tokens || 0).toLocaleString()}`}
                    icon={Zap}
                />
                <StatsCard
                    title="Cache Hits"
                    value={stats?.cache_hits.toLocaleString() || '0'}
                    description="Requests served from cache"
                    icon={Database}
                />
                <StatsCard
                    title="Estimated Cost"
                    value={`$${(stats?.total_cost || 0).toFixed(4)}`}
                    description="Based on token usage"
                    icon={Coins}
                />
            </div>

            <div className="grid gap-6 lg:grid-cols-7">
                <RequestChart data={stats?.hourly_activity || []} />
                <ProviderList
                    providers={config.providers}
                    providerStatuses={providerStatuses}
                    onUpdate={handleProvidersReorder}
                    onEdit={(p) => {
                        setEditingProvider(p);
                        setIsFormOpen(true);
                    }}
                    onDelete={handleDeleteProvider}
                    onAdd={() => {
                        setEditingProvider(undefined);
                        setIsFormOpen(true);
                    }}
                />
            </div>

            <Card>
                <CardHeader>
                    <CardTitle>Recent Requests</CardTitle>
                    <CardDescription>Real-time log of AI interactions</CardDescription>
                </CardHeader>
                <CardContent>
                    <div className="rounded-md border">
                        <div className="grid grid-cols-8 gap-4 p-4 font-medium text-sm bg-muted/50 border-b">
                            <div>Time</div>
                            <div>Client</div>
                            <div>Path</div>
                            <div>Provider</div>
                            <div>Status</div>
                            <div>Duration</div>
                            <div>Tokens</div>
                            <div>Cost</div>
                        </div>
                        <div className="max-h-[300px] overflow-y-auto">
                            {stats?.recent_requests.map((log) => (
                                <div key={log.id} className="grid grid-cols-8 gap-4 p-4 text-sm border-b last:border-0 hover:bg-muted/30 transition-colors">
                                    <div className="text-muted-foreground">
                                        {formatDistanceToNow(new Date(log.timestamp * 1000), { addSuffix: true })}
                                    </div>
                                    <div className="truncate font-mono text-xs" title={log.client_agent}>{log.client_agent}</div>
                                    <div className="truncate font-mono text-xs" title={log.path}>{log.path}</div>
                                    <div className="font-medium">{log.provider}</div>
                                    <div>
                                        <span className={`px-2 py-0.5 rounded text-xs ${log.status >= 200 && log.status < 300 ? 'bg-green-500/10 text-green-500' : 'bg-red-500/10 text-red-500'}`}>
                                            {log.status}
                                        </span>
                                    </div>
                                    <div>{log.duration_ms}ms</div>
                                    <div>{(log.input_tokens + log.output_tokens).toLocaleString()}</div>
                                    <div>${log.cost.toFixed(6)}</div>
                                </div>
                            ))}
                            {(!stats?.recent_requests || stats.recent_requests.length === 0) && (
                                <div className="p-8 text-center text-muted-foreground">
                                    No requests recorded yet.
                                </div>
                            )}
                        </div>
                    </div>
                </CardContent>
            </Card>

            <Dialog open={isFormOpen} onOpenChange={setIsFormOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>{editingProvider ? 'Edit Provider' : 'Add Provider'}</DialogTitle>
                    </DialogHeader>
                    <ProviderForm
                        initialData={editingProvider || undefined}
                        onSubmit={editingProvider ? handleEditProvider : handleAddProvider}
                        onCancel={() => setIsFormOpen(false)}
                    />
                </DialogContent>
            </Dialog>
        </div>
    );
}
