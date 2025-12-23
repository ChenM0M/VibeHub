import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Provider, ApiType } from '@/types/gateway';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Plus, X } from 'lucide-react';

interface ProviderFormProps {
    initialData?: Partial<Provider>;
    onSubmit: (data: Provider) => void;
    onCancel: () => void;
}

const API_TYPE_OPTIONS: { value: ApiType; label: string }[] = [
    { value: 'Anthropic', label: 'Claude Code (Anthropic)' },
    { value: 'OpenAIResponses', label: 'CodeX (OpenAI Responses)' },
    { value: 'OpenAIChat', label: 'OpenAI Chat (Cline等)' },
];

export function ProviderForm({ initialData, onSubmit, onCancel }: ProviderFormProps) {
    const { t } = useTranslation();
    const [formData, setFormData] = useState<Partial<Provider>>({
        name: '',
        base_url: 'https://api.openai.com',
        api_key: '',
        enabled: true,
        model_mapping: {},
        api_types: ['OpenAIChat'],
        weight: 100,
        input_price_per_1k: 0.003,
        output_price_per_1k: 0.015,
        claude_code_proxy: false,
        ...initialData
    });

    // 模型映射编辑状态
    const [newMappingSource, setNewMappingSource] = useState('');
    const [newMappingTarget, setNewMappingTarget] = useState('');

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        // 如果启用了 claude_code_proxy，确保 Anthropic 在 api_types 中
        let apiTypes = formData.api_types || ['OpenAIChat'];
        if (formData.claude_code_proxy && !apiTypes.includes('Anthropic')) {
            apiTypes = ['Anthropic', ...apiTypes];
        }
        onSubmit({
            id: initialData?.id || crypto.randomUUID(),
            name: formData.name || 'New Provider',
            base_url: formData.base_url || '',
            api_key: formData.api_key || '',
            model_mapping: formData.model_mapping || {},
            enabled: formData.enabled ?? true,
            api_types: apiTypes,
            weight: formData.weight || 100,
            input_price_per_1k: formData.input_price_per_1k || 0,
            output_price_per_1k: formData.output_price_per_1k || 0,
            claude_code_proxy: formData.claude_code_proxy || false,
        });
    };

    const handleApiTypeToggle = (apiType: ApiType, checked: boolean) => {
        const currentTypes = formData.api_types || [];
        if (checked) {
            setFormData({ ...formData, api_types: [...currentTypes, apiType] });
        } else {
            setFormData({ ...formData, api_types: currentTypes.filter(t => t !== apiType) });
        }
    };

    const handleAddMapping = () => {
        if (newMappingSource.trim() && newMappingTarget.trim()) {
            setFormData({
                ...formData,
                model_mapping: {
                    ...formData.model_mapping,
                    [newMappingSource.trim()]: newMappingTarget.trim()
                }
            });
            setNewMappingSource('');
            setNewMappingTarget('');
        }
    };

    const handleRemoveMapping = (source: string) => {
        const newMapping = { ...formData.model_mapping };
        delete newMapping[source];
        setFormData({ ...formData, model_mapping: newMapping });
    };

    return (
        <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
                <Label htmlFor="name">{t('gateway.form.name')}</Label>
                <Input
                    id="name"
                    value={formData.name}
                    onChange={e => setFormData({ ...formData, name: e.target.value })}
                    placeholder={t('gateway.form.namePlaceholder')}
                    required
                />
            </div>
            <div className="space-y-2">
                <Label htmlFor="base_url">{t('gateway.form.baseUrl')}</Label>
                <Input
                    id="base_url"
                    value={formData.base_url}
                    onChange={e => setFormData({ ...formData, base_url: e.target.value })}
                    placeholder={t('gateway.form.baseUrlPlaceholder')}
                    required
                />
            </div>
            <div className="space-y-2">
                <Label htmlFor="api_key">{t('gateway.form.apiKey')}</Label>
                <Input
                    id="api_key"
                    type="password"
                    value={formData.api_key}
                    onChange={e => setFormData({ ...formData, api_key: e.target.value })}
                    placeholder={t('gateway.form.apiKeyPlaceholder')}
                />
            </div>

            <div className="space-y-2">
                <Label>{t('gateway.form.supportedApiTypes')}</Label>
                <div className="flex flex-col gap-2">
                    {API_TYPE_OPTIONS.map(option => (
                        <div key={option.value} className="flex items-center space-x-2">
                            <Checkbox
                                id={`api-type-${option.value}`}
                                checked={formData.api_types?.includes(option.value)}
                                onCheckedChange={(checked) => handleApiTypeToggle(option.value, !!checked)}
                            />
                            <Label htmlFor={`api-type-${option.value}`} className="font-normal">
                                {option.label}
                            </Label>
                        </div>
                    ))}
                </div>
            </div>

            <div className="space-y-2 border rounded-lg p-3 bg-muted/30">
                <div className="flex items-center space-x-2">
                    <Checkbox
                        id="claude_code_proxy"
                        checked={formData.claude_code_proxy}
                        onCheckedChange={(checked) => setFormData({ ...formData, claude_code_proxy: !!checked })}
                    />
                    <Label htmlFor="claude_code_proxy" className="font-medium">
                        {t('gateway.form.claudeCodeProxy')}
                    </Label>
                </div>
                <p className="text-xs text-muted-foreground ml-6">
                    {t('gateway.form.claudeCodeProxyDesc')}
                </p>
            </div>

            {/* 模型映射区块 */}
            <div className="space-y-2 border rounded-lg p-3 bg-muted/30">
                <Label className="font-medium">{t('gateway.form.modelMapping', '模型映射')}</Label>
                <p className="text-xs text-muted-foreground">
                    {t('gateway.form.modelMappingDesc', '将请求的模型名映射到目标模型（如 claude-3-haiku → claude-3-5-sonnet）')}
                </p>

                {/* 现有映射列表 */}
                {Object.entries(formData.model_mapping || {}).length > 0 && (
                    <div className="space-y-1 mt-2">
                        {Object.entries(formData.model_mapping || {}).map(([source, target]) => (
                            <div key={source} className="flex items-center gap-2 text-sm bg-background/50 rounded px-2 py-1">
                                <Input
                                    value={source}
                                    onChange={e => {
                                        const newSource = e.target.value.trim();
                                        if (newSource && newSource !== source) {
                                            const newMapping = { ...formData.model_mapping };
                                            delete newMapping[source];
                                            newMapping[newSource] = target;
                                            setFormData({ ...formData, model_mapping: newMapping });
                                        }
                                    }}
                                    className="h-7 text-xs font-mono flex-1"
                                />
                                <span className="text-muted-foreground shrink-0">→</span>
                                <Input
                                    value={target}
                                    onChange={e => {
                                        const newTarget = e.target.value;
                                        setFormData({
                                            ...formData,
                                            model_mapping: {
                                                ...formData.model_mapping,
                                                [source]: newTarget
                                            }
                                        });
                                    }}
                                    className="h-7 text-xs font-mono flex-1"
                                />
                                <Button
                                    type="button"
                                    variant="ghost"
                                    size="icon"
                                    className="h-6 w-6 text-destructive/70 hover:text-destructive shrink-0"
                                    onClick={() => handleRemoveMapping(source)}
                                >
                                    <X className="h-3 w-3" />
                                </Button>
                            </div>
                        ))}
                    </div>
                )}

                {/* 添加新映射 */}
                <div className="flex items-center gap-2 mt-2">
                    <Input
                        placeholder={t('gateway.form.sourceModel', '源模型')}
                        value={newMappingSource}
                        onChange={e => setNewMappingSource(e.target.value)}
                        className="h-8 text-xs"
                    />
                    <span className="text-muted-foreground">→</span>
                    <Input
                        placeholder={t('gateway.form.targetModel', '目标模型')}
                        value={newMappingTarget}
                        onChange={e => setNewMappingTarget(e.target.value)}
                        className="h-8 text-xs"
                    />
                    <Button
                        type="button"
                        variant="outline"
                        size="icon"
                        className="h-8 w-8 shrink-0"
                        onClick={handleAddMapping}
                        disabled={!newMappingSource.trim() || !newMappingTarget.trim()}
                    >
                        <Plus className="h-4 w-4" />
                    </Button>
                </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                <div className="space-y-2">
                    <Label htmlFor="weight">{t('gateway.form.weight')}</Label>
                    <Input
                        id="weight"
                        type="number"
                        value={formData.weight}
                        onChange={e => setFormData({ ...formData, weight: parseInt(e.target.value) || 100 })}
                        placeholder="100"
                    />
                </div>
                <div className="space-y-2">
                    <Label htmlFor="input_price">{t('gateway.form.inputPrice')}</Label>
                    <Input
                        id="input_price"
                        type="number"
                        step="0.0001"
                        value={formData.input_price_per_1k}
                        onChange={e => setFormData({ ...formData, input_price_per_1k: parseFloat(e.target.value) || 0 })}
                        placeholder="0.003"
                    />
                </div>
                <div className="space-y-2">
                    <Label htmlFor="output_price">{t('gateway.form.outputPrice')}</Label>
                    <Input
                        id="output_price"
                        type="number"
                        step="0.0001"
                        value={formData.output_price_per_1k}
                        onChange={e => setFormData({ ...formData, output_price_per_1k: parseFloat(e.target.value) || 0 })}
                        placeholder="0.015"
                    />
                </div>
            </div>

            <div className="flex justify-end gap-2 pt-4">
                <Button type="button" variant="outline" onClick={onCancel}>
                    {t('gateway.form.cancel')}
                </Button>
                <Button type="submit">
                    {t('gateway.form.save')}
                </Button>
            </div>
        </form>
    );
}

