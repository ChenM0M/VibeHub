import type { AgentKind, ModelProfile } from '@/v3/contracts/generated';
import type { UpstreamModelMetadata } from '@/services/tauri';

export function modelFromUpstream(agent: AgentKind, model: UpstreamModelMetadata): ModelProfile {
    const effort = model.supports_effort === true ? [...(model.effort_options || [])] : [];
    return {
        model_id: model.model_id,
        display_name: model.display_name || model.model_id,
        enabled: true,
        thinking: {
            supports_reasoning: model.supports_reasoning ?? null,
            supports_effort: model.supports_effort ?? null,
            selected: null,
            options: agent === 'codex' ? effort : [],
            effort_options: effort,
            thinking_types: model.thinking_types || [],
            custom_allowed: false,
            variant_values: null,
            variant_values_changed: false,
        },
    };
}
