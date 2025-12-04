import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Provider, ApiType } from '@/types/gateway';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';

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
        ...initialData
    });

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        onSubmit({
            id: initialData?.id || crypto.randomUUID(),
            name: formData.name || 'New Provider',
            base_url: formData.base_url || '',
            api_key: formData.api_key || '',
            model_mapping: formData.model_mapping || {},
            enabled: formData.enabled ?? true,
            api_types: formData.api_types || ['OpenAIChat'],
            weight: formData.weight || 100,
            input_price_per_1k: formData.input_price_per_1k || 0,
            output_price_per_1k: formData.output_price_per_1k || 0,
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

            <div className="grid grid-cols-3 gap-4">
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
