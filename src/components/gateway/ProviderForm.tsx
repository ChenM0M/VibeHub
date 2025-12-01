import React, { useState } from 'react';
import { Provider } from '@/types/gateway';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';

interface ProviderFormProps {
    initialData?: Partial<Provider>;
    onSubmit: (data: Provider) => void;
    onCancel: () => void;
}

export function ProviderForm({ initialData, onSubmit, onCancel }: ProviderFormProps) {
    const [formData, setFormData] = useState<Partial<Provider>>({
        name: '',
        base_url: 'https://api.openai.com/v1',
        api_key: '',
        enabled: true,
        model_mapping: {},
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
        });
    };

    return (
        <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                    id="name"
                    value={formData.name}
                    onChange={e => setFormData({ ...formData, name: e.target.value })}
                    placeholder="e.g. OpenAI"
                    required
                />
            </div>
            <div className="space-y-2">
                <Label htmlFor="base_url">Base URL</Label>
                <Input
                    id="base_url"
                    value={formData.base_url}
                    onChange={e => setFormData({ ...formData, base_url: e.target.value })}
                    placeholder="https://api.openai.com/v1"
                    required
                />
            </div>
            <div className="space-y-2">
                <Label htmlFor="api_key">API Key</Label>
                <Input
                    id="api_key"
                    type="password"
                    value={formData.api_key}
                    onChange={e => setFormData({ ...formData, api_key: e.target.value })}
                    placeholder="sk-..."
                />
            </div>
            <div className="flex justify-end gap-2 pt-4">
                <Button type="button" variant="outline" onClick={onCancel}>
                    Cancel
                </Button>
                <Button type="submit">
                    Save
                </Button>
            </div>
        </form>
    );
}
