import type { AgentProfileDocument } from '@/v3/contracts/generated';

export function modelIsDefault(profile: AgentProfileDocument, providerId: string, modelId: string): boolean {
    const { default_model_id: model, default_provider_id: provider } = profile.managed;
    return model === `${providerId}/${modelId}`
        || (model === modelId && (!provider || provider === providerId));
}

/** Called after updating the model list in a cloned draft. */
export function updateDefaultAfterModelEdit(
    draft: AgentProfileDocument, providerId: string, modelId: string, enabled: boolean, wasDefault: boolean,
): void {
    const key = (provider: string, model: string) => draft.agent === 'opencode' ? `${provider}/${model}` : model;
    if (draft.agent === 'claude_code') {
        draft.managed.default_provider_id = enabled ? providerId : null;
        draft.managed.default_model_id = enabled ? modelId : null;
    } else if (enabled && (wasDefault || !draft.managed.default_model_id)) {
        draft.managed.default_provider_id = providerId;
        draft.managed.default_model_id = key(providerId, modelId);
    } else if (!enabled && wasDefault) {
        const provider = draft.managed.providers.find((item) => item.models.some((model) => model.enabled));
        const model = provider?.models.find((item) => item.enabled);
        draft.managed.default_provider_id = provider?.provider_id || null;
        draft.managed.default_model_id = provider && model ? key(provider.provider_id, model.model_id) : null;
    }
}
