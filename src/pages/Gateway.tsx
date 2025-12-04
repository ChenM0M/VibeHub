import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Server, Coins, Database, Zap, Bot, MessageSquare, Code2, Copy, Check } from 'lucide-react';
import { GatewayConfig, Provider, GatewayStats } from '@/types/gateway';
import { ProviderForm } from '@/components/gateway/ProviderForm';
import { StatsCard } from '@/components/gateway/StatsCard';
import { RequestChart } from '@/components/gateway/RequestChart';
import { ProviderList } from '@/components/gateway/ProviderList';
import { formatDistanceToNow } from 'date-fns';
import { Badge } from '@/components/ui/badge';

interface ProviderStatusEvent {
    provider_id: string;
    status: string;
    api_type: string;
}

export function Gateway() {
    const { t } = useTranslation();
    const [config, setConfig] = useState<GatewayConfig | null>(null);
    const [stats, setStats] = useState<GatewayStats | null>(null);
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingProvider, setEditingProvider] = useState<Provider | undefined>(undefined);
    const [providerStatuses, setProviderStatuses] = useState<Record<string, string>>({});
    const [copiedPort, setCopiedPort] = useState<string | null>(null);

    const copyToClipboard = async (port: number) => {
        const url = `http://localhost:${port}`;
        await navigator.clipboard.writeText(url);
        setCopiedPort(String(port));
        setTimeout(() => setCopiedPort(null), 2000);
    };

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

        const unlistenPromise = listen<ProviderStatusEvent>('gateway://provider-status', (event) => {
            setProviderStatuses(prev => ({
                ...prev,
                [event.payload.provider_id]: event.payload.status
            }));

            if (event.payload.status !== 'pending') {
                setTimeout(() => {
                    setProviderStatuses(prev => {
                        const next = { ...prev };
                        delete next[event.payload.provider_id];
                        return next;
                    });
                }, 3000);
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

    const handleToggleGateway = (type: 'anthropic' | 'responses' | 'chat', enabled: boolean) => {
        if (!config) return;
        const key = `${type}_enabled` as keyof GatewayConfig;
        handleSaveConfig({ ...config, [key]: enabled });
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

    if (!config) return <div className="p-8">{t('common.loading')}</div>;

    const cacheHitRate = stats && (stats.cache_hits + stats.cache_misses) > 0
        ? ((stats.cache_hits / (stats.cache_hits + stats.cache_misses)) * 100).toFixed(1)
        : '0';

    return (
        <div className="space-y-6 p-4 md:p-6 lg:p-8 max-w-[1600px] mx-auto overflow-x-hidden">
            <div className="flex items-center justify-between">
                <div>
                    <h1 className="text-3xl font-bold tracking-tight">{t('gateway.title')}</h1>
                    <p className="text-muted-foreground mt-2">
                        {t('gateway.subtitle')} <span className="text-green-600 dark:text-green-400 font-medium">{t('gateway.noSecretNeeded')}</span>
                    </p>
                </div>
            </div>

            {/* Gateway Status Cards */}
            <div className="grid gap-4 md:grid-cols-3">
                {/* Anthropic Gateway */}
                <Card className={config.anthropic_enabled ? 'border-blue-500/30 bg-blue-500/5' : ''}>
                    <CardHeader className="pb-2">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <Bot className="h-5 w-5 text-blue-500" />
                                <CardTitle className="text-base">{t('gateway.claudeCode')}</CardTitle>
                            </div>
                            <Switch
                                checked={config.anthropic_enabled}
                                onCheckedChange={(v) => handleToggleGateway('anthropic', v)}
                            />
                        </div>
                    </CardHeader>
                    <CardContent>
                        <div className="flex items-center justify-between">
                            <button
                                onClick={() => copyToClipboard(config.anthropic_port)}
                                className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors group"
                                title={t('gateway.copyUrl')}
                            >
                                <code>:{config.anthropic_port}</code>
                                {copiedPort === String(config.anthropic_port) ? (
                                    <Check className="h-3.5 w-3.5 text-green-500" />
                                ) : (
                                    <Copy className="h-3.5 w-3.5 opacity-0 group-hover:opacity-100 transition-opacity" />
                                )}
                            </button>
                            <Badge variant={config.anthropic_enabled ? "default" : "secondary"}>
                                {config.anthropic_enabled ? t('common.running') : t('common.stopped')}
                            </Badge>
                        </div>
                        <div className="text-xs text-muted-foreground mt-2">
                            {t('common.requests')}: {stats?.anthropic_requests || 0}
                        </div>
                    </CardContent>
                </Card>

                {/* OpenAI Responses Gateway */}
                <Card className={config.responses_enabled ? 'border-green-500/30 bg-green-500/5' : ''}>
                    <CardHeader className="pb-2">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <Code2 className="h-5 w-5 text-green-500" />
                                <CardTitle className="text-base">{t('gateway.codex')}</CardTitle>
                            </div>
                            <Switch
                                checked={config.responses_enabled}
                                onCheckedChange={(v) => handleToggleGateway('responses', v)}
                            />
                        </div>
                    </CardHeader>
                    <CardContent>
                        <div className="flex items-center justify-between">
                            <button
                                onClick={() => copyToClipboard(config.responses_port)}
                                className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors group"
                                title={t('gateway.copyUrl')}
                            >
                                <code>:{config.responses_port}</code>
                                {copiedPort === String(config.responses_port) ? (
                                    <Check className="h-3.5 w-3.5 text-green-500" />
                                ) : (
                                    <Copy className="h-3.5 w-3.5 opacity-0 group-hover:opacity-100 transition-opacity" />
                                )}
                            </button>
                            <Badge variant={config.responses_enabled ? "default" : "secondary"}>
                                {config.responses_enabled ? t('common.running') : t('common.stopped')}
                            </Badge>
                        </div>
                        <div className="text-xs text-muted-foreground mt-2">
                            {t('common.requests')}: {stats?.responses_requests || 0}
                        </div>
                    </CardContent>
                </Card>

                {/* OpenAI Chat Gateway */}
                <Card className={config.chat_enabled ? 'border-yellow-500/30 bg-yellow-500/5' : ''}>
                    <CardHeader className="pb-2">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <MessageSquare className="h-5 w-5 text-yellow-500" />
                                <CardTitle className="text-base">{t('gateway.openaiChat')}</CardTitle>
                            </div>
                            <Switch
                                checked={config.chat_enabled}
                                onCheckedChange={(v) => handleToggleGateway('chat', v)}
                            />
                        </div>
                    </CardHeader>
                    <CardContent>
                        <div className="flex items-center justify-between">
                            <button
                                onClick={() => copyToClipboard(config.chat_port)}
                                className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors group"
                                title={t('gateway.copyUrl')}
                            >
                                <code>:{config.chat_port}</code>
                                {copiedPort === String(config.chat_port) ? (
                                    <Check className="h-3.5 w-3.5 text-green-500" />
                                ) : (
                                    <Copy className="h-3.5 w-3.5 opacity-0 group-hover:opacity-100 transition-opacity" />
                                )}
                            </button>
                            <Badge variant={config.chat_enabled ? "default" : "secondary"}>
                                {config.chat_enabled ? t('common.running') : t('common.stopped')}
                            </Badge>
                        </div>
                        <div className="text-xs text-muted-foreground mt-2">
                            {t('common.requests')}: {stats?.chat_requests || 0}
                        </div>
                    </CardContent>
                </Card>
            </div>

            {/* Stats Cards */}
            <div className="grid gap-4 grid-cols-2 lg:grid-cols-4">
                <StatsCard
                    title={t('gateway.totalRequests')}
                    value={stats?.total_requests.toLocaleString() || '0'}
                    description={t('gateway.allGateways')}
                    icon={Server}
                />
                <StatsCard
                    title={t('gateway.tokenUsage')}
                    value={((stats?.total_input_tokens || 0) + (stats?.total_output_tokens || 0)).toLocaleString()}
                    description={`${t('gateway.input')}: ${(stats?.total_input_tokens || 0).toLocaleString()} / ${t('gateway.output')}: ${(stats?.total_output_tokens || 0).toLocaleString()}`}
                    icon={Zap}
                />
                <StatsCard
                    title={t('gateway.cacheHits')}
                    value={`${stats?.cache_hits.toLocaleString() || '0'} (${cacheHitRate}%)`}
                    description={`${t('gateway.cacheMisses')}: ${stats?.cache_misses || 0}`}
                    icon={Database}
                />
                <StatsCard
                    title={t('gateway.estimatedCost')}
                    value={`$${(stats?.total_cost || 0).toFixed(4)}`}
                    description={t('gateway.basedOnTokens')}
                    icon={Coins}
                />
            </div>

            <div className="grid gap-6 grid-cols-1 xl:grid-cols-2">
                <RequestChart data={stats?.hourly_activity || []} />
                <ProviderList
                    providers={config.providers}
                    providerStatuses={providerStatuses}
                    providerStats={stats?.provider_stats || {}}
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
                    <CardTitle>{t('gateway.recentRequests')}</CardTitle>
                    <CardDescription>{t('gateway.realtimeLog')}</CardDescription>
                </CardHeader>
                <CardContent>
                    <div className="overflow-x-auto">
                        <div className="grid grid-cols-9 gap-4 p-4 font-medium text-sm bg-muted/50 border-b min-w-[900px]">
                            <div>{t('gateway.time')}</div>
                            <div>{t('gateway.type')}</div>
                            <div>{t('gateway.client')}</div>
                            <div>{t('gateway.path')}</div>
                            <div>{t('gateway.provider')}</div>
                            <div>{t('gateway.status')}</div>
                            <div>{t('gateway.latency')}</div>
                            <div>{t('gateway.tokens')}</div>
                            <div>{t('gateway.cost')}</div>
                        </div>
                        <div className="max-h-[300px] overflow-y-auto">
                            {stats?.recent_requests.map((log) => (
                                <div key={log.id} className="grid grid-cols-9 gap-4 p-4 text-sm border-b last:border-0 hover:bg-muted/30 transition-colors">
                                    <div className="text-muted-foreground">
                                        {formatDistanceToNow(new Date(log.timestamp * 1000), { addSuffix: true })}
                                    </div>
                                    <div>
                                        <Badge variant="outline" className="text-[10px]">
                                            {log.api_type || 'unknown'}
                                        </Badge>
                                    </div>
                                    <div className="truncate font-mono text-xs" title={log.client_agent}>{log.client_agent}</div>
                                    <div className="truncate font-mono text-xs" title={log.path}>{log.path}</div>
                                    <div className="font-medium">{log.provider}</div>
                                    <div>
                                        <span className={`px-2 py-0.5 rounded text-xs ${log.status >= 200 && log.status < 300 ? 'bg-green-500/10 text-green-500' : 'bg-red-500/10 text-red-500'}`}>
                                            {log.status}
                                            {log.cached && ' 📦'}
                                        </span>
                                    </div>
                                    <div>{log.duration_ms}ms</div>
                                    <div>{(log.input_tokens + log.output_tokens).toLocaleString()}</div>
                                    <div>${log.cost.toFixed(6)}</div>
                                </div>
                            ))}
                            {(!stats?.recent_requests || stats.recent_requests.length === 0) && (
                                <div className="p-8 text-center text-muted-foreground">
                                    {t('gateway.noRequests')}
                                </div>
                            )}
                        </div>
                    </div>
                </CardContent>
            </Card>

            <Dialog open={isFormOpen} onOpenChange={setIsFormOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>{editingProvider ? t('gateway.form.editTitle') : t('gateway.form.addTitle')}</DialogTitle>
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
