import type { AppConfig } from '@/types';
import type {
    AgentProfileDiscoverResult,
    AgentProfileDocument,
    RuntimeTarget,
} from '@/v3/contracts/generated';
import { viteEnv } from '@/lib/viteEnv';

export function isReleaseFixture(): boolean {
    return Boolean(viteEnv.DEV) && new URLSearchParams(window.location.search).get('fixture') === 'release';
}

const demoPath = (native: string) => ({
    platform: 'macos' as const,
    native,
    display: native,
    identity_key: native,
    path_kind: 'absolute' as const,
    accessible: true,
});

export const releaseFixtureTarget: RuntimeTarget = {
    target_id: 'runtime.macos.host',
    kind: 'host',
    platform: 'macos',
    distribution: null,
    display_name: 'macOS host',
    home_path: demoPath('/Demo/Home'),
    source: 'fixture',
    capability_manifest_revision: null,
};

export const releaseFixtureProfile: AgentProfileDocument = {
    profile_id: 'opencode.profile.demo',
    display_name: 'opencode.json',
    agent: 'opencode',
    runtime_target_id: releaseFixtureTarget.target_id,
    source: {
        path: demoPath('/Demo/Home/.config/opencode/opencode.json'),
        format: 'json',
        scope: 'user',
        profile_name: null,
        last_modified: '2026-08-18T00:00:00Z',
    },
    revision: {
        revision: 1,
        content_sha256: '0'.repeat(64),
        observed_at: '2026-08-18T00:00:00Z',
    },
    managed: {
        providers: [
            {
                provider_id: 'openai',
                display_name: 'OpenAI',
                base_url: 'https://api.openai.com/v1',
                credential: {
                    kind: 'secret_store',
                    reference: 'options.apiKey',
                    display: '配置内存在 API Key（已隐藏）',
                    secret_state: 'configured',
                    persisted_in_config: false,
                },
                protocol: {
                    native_protocol: 'openai_chat_completions',
                    upstream_protocol: 'openai_chat_completions',
                    route: 'direct',
                    compatibility: 'supported',
                    adapter_id: null,
                    adapter_version: null,
                    limitations: [],
                },
                models: [
                    {
                        model_id: 'gpt-5.4',
                        display_name: 'GPT-5.4',
                        enabled: true,
                        thinking: {
                            supports_reasoning: true,
                            supports_effort: true,
                            selected: 'medium',
                            options: ['low', 'medium', 'high', 'xhigh'],
                            custom_allowed: false,
                        },
                    },
                    {
                        model_id: 'gpt-5.4-mini',
                        display_name: 'GPT-5.4 mini',
                        enabled: true,
                        thinking: {
                            supports_reasoning: true,
                            supports_effort: true,
                            selected: 'low',
                            options: ['low', 'medium', 'high'],
                            custom_allowed: false,
                        },
                    },
                ],
            },
        ],
        default_provider_id: 'openai',
        default_model_id: 'openai/gpt-5.4',
        small_model_id: 'openai/gpt-5.4-mini',
    },
    default_state: {
        is_default: true,
        selected_by: 'native',
        projection: {
            strategy: 'none',
            target_path: null,
            managed_field_paths: [],
            warning: null,
        },
    },
    preservation: {
        unknown_fields_preserved: true,
        comments_preserved: false,
        formatting_preserved: true,
        managed_field_paths: [],
        unmanaged_field_paths: [],
        backup_path: null,
        rollback_available: false,
    },
    protocol: {
        native_protocol: 'openai_chat_completions',
        upstream_protocol: 'openai_chat_completions',
        route: 'direct',
        compatibility: 'supported',
        adapter_id: null,
        adapter_version: null,
        limitations: [],
    },
    schema_capability: {
        schema_id: 'opencode',
        schema_version: '1.0',
        compatibility: 'supported',
        supported_fields: [],
        unsupported_fields: [],
        unknown_fields: [],
    },
    launch: {
        executable: 'opencode',
        profile_argument: null,
        settings_argument: null,
        extra_arguments: [],
    },
};

// Development-only Claude regression fixture; no user configuration is read or written.
if (isReleaseFixture() && new URLSearchParams(window.location.search).get('fixtureAgent') === 'claude') {
    Object.assign(releaseFixtureProfile, {
        agent: 'claude_code', profile_id: 'claude.profile.demo', display_name: 'Claude regression',
        source: { ...releaseFixtureProfile.source, scope: 'profile', path: demoPath('/Demo/Home/.claude/vibehub-profiles/demo.settings.json') },
    });
    const provider = releaseFixtureProfile.managed.providers[0];
    provider.provider_id = 'anthropic';
    provider.display_name = 'Anthropic fixture';
    provider.base_url = 'https://example.invalid';
    provider.protocol.native_protocol = 'anthropic_messages';
    provider.protocol.upstream_protocol = 'anthropic_messages';
    provider.models = [{ ...provider.models[0], model_id: 'demo-model', display_name: 'Demo model' }];
    provider.models[0].thinking = { supports_reasoning: true, supports_effort: false, selected: null, options: ['enabled', 'disabled'], custom_allowed: false };
    releaseFixtureProfile.managed.default_provider_id = 'anthropic';
    releaseFixtureProfile.managed.default_model_id = 'demo-model';
    releaseFixtureProfile.managed.small_model_id = null;
    releaseFixtureProfile.protocol = { ...provider.protocol };
    releaseFixtureProfile.schema_capability.schema_id = 'claude-code.settings';
    releaseFixtureProfile.launch = { executable: 'claude', profile_argument: null, settings_argument: '--settings', extra_arguments: ['--setting-sources', ''] };
    releaseFixtureProfile.managed.claude_advanced = { haiku_model: 'legacy-model', small_fast_model: null };
    releaseFixtureProfile.schema_capability.capability_declaration = {
        status: 'declared', source: 'claude-code.settings.adapter', version: '1.0',
        supported_fields: ['model', 'env.ANTHROPIC_DEFAULT_HAIKU_MODEL'],
        fallback_priority: ['explicit_override', 'claude_native_resolution'], message: null,
    };
    releaseFixtureProfile.schema_capability.custom_model_options = {
        status: 'available', source: 'managed.providers[].models', values: ['demo-model'],
        allow_custom: true, fallback_priority: ['explicit_override', 'claude_native_resolution'], message: null,
    };
}

export const releaseFixtureDiscovery: AgentProfileDiscoverResult = {
    kind: 'agent_profile_discover_result',
    schema_version: '1.0',
    generated_at: '2026-08-18T00:00:00Z',
    model_version: 'v3-agent-profile-commands-1',
    freshness: 'fresh',
    completeness: 'complete',
    evidence_refs: [],
    warnings: [],
    errors: [],
    agent: releaseFixtureProfile.agent,
    runtime_targets: [releaseFixtureTarget],
    profiles: [
        {
            profile_id: releaseFixtureProfile.profile_id,
            display_name: releaseFixtureProfile.display_name,
            agent: releaseFixtureProfile.agent,
            runtime_target_id: releaseFixtureTarget.target_id,
            source_path: releaseFixtureProfile.source.path,
            revision: releaseFixtureProfile.revision,
            is_default: true,
            compatibility: 'supported',
        },
    ],
    default_profile_id: releaseFixtureProfile.profile_id,
};

export const releaseFixtureUpstreamModels = [
    { model_id: 'gpt-5.4', display_name: 'GPT-5.4', imported: true },
    { model_id: 'gpt-5.4-mini', display_name: 'GPT-5.4 mini', imported: true },
    { model_id: 'gpt-4.1', display_name: 'GPT-4.1' },
    { model_id: 'gpt-4o', display_name: 'GPT-4o' },
    { model_id: 'o4-mini', display_name: 'o4-mini' },
];

export const releaseFixtureConfig: AppConfig = {
    workspaces: [
        {
            id: 'ws-demo',
            name: '产品工作区',
            path: '/Demo/Workspace',
            auto_scan: false,
            created_at: '2026-05-01T00:00:00.000Z',
        },
    ],
    tags: [
        {
            id: 'tag-claude-daily',
            name: 'Claude Daily',
            color: '#D97757',
            category: 'cli',
            config: {
                executable: 'claude',
                args: ['--settings', '/Demo/Home/.claude/vibehub-profiles/VibeHub Daily.settings.json'],
                terminal: 'Terminal',
            },
        },
    ],
    projects: [],
    theme: 'light',
    recent_projects: [],
};
