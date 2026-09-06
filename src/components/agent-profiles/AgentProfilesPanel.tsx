import React, { useEffect, useMemo, useRef, useState } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
    AlertCircle,
    Check,
    ChevronDown,
    ChevronRight,
    Cloud,
    Copy,
    FileCode2,
    Gauge,
    KeyRound,
    Laptop,
    Loader2,
    Pencil,
    Plus,
    RefreshCw,
    Save,
    ScanSearch,
    Server,
    Settings2,
    Sparkles,
    Terminal,
    Trash2,
} from 'lucide-react';
import { ClaudeBrandIcon, CodexBrandIcon, OpenCodeBrandIcon } from '@/components/AgentBrandIcons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';
import { useDevScreenshotCue } from '@/lib/devScreenshotCue';
import {
    isReleaseFixture,
    releaseFixtureDiscovery,
    releaseFixtureProfile,
    releaseFixtureTarget,
    releaseFixtureUpstreamModels,
} from '@/lib/releaseFixture';
import { tauriApi, V3AgentProfileSaveRequest } from '@/services/tauri';
import { CLAUDE_AUTO_MODEL_VALUE, normalizeClaudeModelSelection } from './claudeProfile';
import type {
    AgentKind,
    AgentProfileDiscoverResult,
    AgentProfileDocument,
    AgentProfileSaveResult,
    AgentProfileSummary,
    CredentialReference,
    ModelProfile,
    ProtocolCapability,
    ProviderProfile,
    RuntimeTarget,
} from '@/v3/contracts/generated';

const AGENTS: Array<{ id: AgentKind; label: string; descriptionKey: string; icon: React.ElementType }> = [
    { id: 'opencode', label: 'OpenCode', descriptionKey: 'agentProfiles.agents.opencodeDescription', icon: OpenCodeBrandIcon },
    { id: 'claude_code', label: 'Claude Code', descriptionKey: 'agentProfiles.agents.claudeCodeDescription', icon: ClaudeBrandIcon },
    { id: 'codex', label: 'Codex', descriptionKey: 'agentProfiles.agents.codexDescription', icon: CodexBrandIcon },
];

const fieldClass = 'flex h-10 w-full rounded-md border border-input/80 bg-background px-3 py-2 text-sm shadow-sm outline-none transition-[border-color,box-shadow,background-color] duration-150 focus-visible:border-primary/40 focus-visible:ring-2 focus-visible:ring-ring/20';
const mutedFieldClass = `${fieldClass} text-muted-foreground`;
const interactionGroupClass = '[&_button]:transform-gpu [&_button]:transition-[color,background-color,border-color,box-shadow,opacity,transform] [&_button]:duration-150 [&_button]:ease-out [&_button]:active:scale-[0.97]';

type PanelNotice = { kind: 'success' | 'error' | 'info'; text: string };
type ProfileDialogKind = 'create' | 'clone' | 'rename' | 'delete';
type EntityDelete = { kind: 'provider' | 'model'; providerId: string; modelId?: string; label: string };
type ProviderEditor = { mode: 'create' | 'edit'; providerId?: string };
type ModelEditor = { mode: 'create' | 'edit'; providerId: string; modelId?: string };
type ModelImportItem = { model_id: string; display_name: string; imported: boolean };
type ModelImportState = {
    providerId: string;
    loading: boolean;
    error: string | null;
    endpoint: string | null;
    items: ModelImportItem[];
    selected: string[];
    filter: string;
};

type ProviderForm = {
    provider_id: string;
    display_name: string;
    base_url: string;
    credential_kind: CredentialReference['kind'];
    credential_reference: string;
    credential_display: string;
    api_key: string;
    clear_api_key: boolean;
    protocol: ProtocolCapability;
};

type ModelForm = {
    model_id: string;
    display_name: string;
    enabled: boolean;
    supports_reasoning: boolean;
    supports_effort: boolean;
    selected: string;
    options: string[];
    custom_allowed: boolean;
    variant_values: Record<string, unknown> | null;
};

function cloneProfile(profile: AgentProfileDocument): AgentProfileDocument {
    return JSON.parse(JSON.stringify(profile)) as AgentProfileDocument;
}

function attachCredentialWrite(
    profile: AgentProfileDocument,
    secrets: Record<string, string>,
    clears: Record<string, boolean>,
): AgentProfileDocument {
    const next = cloneProfile(profile);
    next.managed.providers = next.managed.providers.map((provider) => {
        const secret = secrets[provider.provider_id];
        const clear = clears[provider.provider_id];
        if (!secret && !clear) return provider;
        return {
            ...provider,
            credential: {
                ...provider.credential,
                ...(secret ? { secret } : {}),
                ...(clear ? { clear_secret: true } : {}),
            } as CredentialReference,
        };
    });
    return next;
}

// Structured config-location errors (CONFIG_* / OPENCODE_CONFIG_*) carry a
// code, path and recovery hint from the Rust core. Render them as a precise
// diagnostic instead of a bare path or a raw JSON dump.
function structuredConfigError(error: unknown): string | null {
    if (!error || typeof error !== 'object') return null;
    const candidate = error as {
        code?: string;
        details?: { path?: string; recovery_hint?: string; message?: string };
        path?: string;
        recovery_hint?: string;
    };
    const details = candidate.details ?? {};
    const code = candidate.code ?? '';
    const isConfigError = /^(CONFIG_|OPENCODE_CONFIG_|RUNTIME_|OPENCODE_)/.test(code);
    if (!isConfigError) return null;
    const message = details.message ?? (candidate as { message?: string }).message;
    const path = details.path ?? candidate.path;
    const hint = details.recovery_hint ?? candidate.recovery_hint;
    const parts: string[] = [];
    if (code) parts.push(code);
    if (message) parts.push(message);
    if (path) parts.push(path);
    if (hint) parts.push(hint);
    return parts.length > 0 ? parts.join(' — ') : null;
}

function errorText(error: unknown, desktopRuntimeRequired: string): string {
    if (typeof error === 'string') {
        return error.includes("reading 'invoke'") ? desktopRuntimeRequired : error;
    }
    if (error && typeof error === 'object') {
        const structured = structuredConfigError(error);
        if (structured) return structured;
        const candidate = error as { code?: string; details?: { message?: string }; message?: string };
        const text = candidate.details?.message || candidate.message || candidate.code || JSON.stringify(error);
        return text.includes("reading 'invoke'") ? desktopRuntimeRequired : text;
    }
    return String(error);
}

function detectErrorText(error: unknown, desktopRuntimeRequired: string, translate: (key: string) => string): string {
    const mapped: Record<string, string> = {
        AGENT_PROFILE_API_KEY_REQUIRED: 'agentProfiles.errors.detectApiKeyRequired',
        AGENT_PROFILE_BASE_URL_REQUIRED: 'agentProfiles.errors.detectBaseUrlRequired',
        AGENT_PROFILE_BASE_URL_INVALID: 'agentProfiles.errors.baseUrlInvalid',
        AGENT_PROFILE_UPSTREAM_AUTH_FAILED: 'agentProfiles.errors.detectAuthFailed',
        AGENT_PROFILE_UPSTREAM_UNAVAILABLE: 'agentProfiles.errors.detectUnavailable',
        AGENT_PROFILE_UPSTREAM_RESPONSE_INVALID: 'agentProfiles.errors.detectInvalidResponse',
        AGENT_PROFILE_UPSTREAM_RESPONSE_TOO_LARGE: 'agentProfiles.errors.detectUnavailable',
    };
    const code = detectErrorCode(error);
    if (code && mapped[code]) return translate(mapped[code]);
    return errorText(error, desktopRuntimeRequired);
}

function detectErrorCode(error: unknown): string | undefined {
    if (typeof error === 'string') {
        const match = error.match(/^(AGENT_PROFILE_[A-Z0-9_]+):/);
        return match?.[1];
    }
    if (!error || typeof error !== 'object') return undefined;
    const candidate = error as { code?: string; details?: { code?: string } };
    return candidate.code || candidate.details?.code;
}

function listModelsProtocol(agent: AgentKind, provider: ProviderProfile): string {
    for (const protocol of [provider.protocol.upstream_protocol, provider.protocol.native_protocol]) {
        if (protocol && protocol !== 'unknown') return protocol;
    }
    return agent === 'claude_code' ? 'anthropic_messages' : 'openai_responses';
}

function modelFromUpstream(agent: AgentKind, modelId: string, displayName: string): ModelProfile {
    const defaults = emptyModelForm(agent);
    return {
        model_id: modelId,
        display_name: displayName || modelId,
        enabled: true,
        thinking: {
            supports_reasoning: defaults.supports_reasoning,
            supports_effort: defaults.supports_effort,
            selected: defaults.options[0] || null,
            options: [...defaults.options],
            custom_allowed: defaults.custom_allowed,
            variant_values: null,
            variant_values_changed: agent === 'opencode' && defaults.options.length > 0,
        },
    };
}

function importUpstreamModelsIntoDraft(
    profile: AgentProfileDocument,
    providerId: string,
    models: Array<{ model_id: string; display_name: string }>,
): AgentProfileDocument {
    const next = cloneProfile(profile);
    const provider = next.managed.providers.find((item) => item.provider_id === providerId);
    if (!provider) return profile;
    const existing = new Set(provider.models.map((model) => model.model_id));
    const added: ModelProfile[] = [];
    for (const model of models) {
        const modelId = model.model_id.trim();
        if (!modelId || existing.has(modelId)) continue;
        existing.add(modelId);
        added.push(modelFromUpstream(next.agent, modelId, model.display_name.trim() || modelId));
    }
    if (added.length === 0) return profile;
    provider.models.push(...added);
    if (!next.managed.default_model_id && added[0]) {
        next.managed.default_provider_id = providerId;
        next.managed.default_model_id = next.agent === 'opencode' ? profileModelKey(providerId, added[0].model_id) : added[0].model_id;
    }
    return next;
}

function agentLabel(agent: AgentKind): string {
    return AGENTS.find((item) => item.id === agent)?.label || agent;
}

function profileModelKey(providerId: string, modelId: string): string {
    return `${providerId}/${modelId}`;
}

function modelIsDefault(profile: AgentProfileDocument, providerId: string, modelId: string): boolean {
    const current = profile.managed.default_model_id;
    return current === modelId || current === profileModelKey(providerId, modelId);
}

function updateManagedModel(profile: AgentProfileDocument, providerId: string, modelId: string): AgentProfileDocument {
    const next = cloneProfile(profile);
    next.managed.default_provider_id = providerId;
    next.managed.default_model_id = next.agent === 'opencode' ? profileModelKey(providerId, modelId) : modelId;
    return next;
}

function updateThinking(profile: AgentProfileDocument, providerId: string, modelId: string, selected: string): AgentProfileDocument {
    const next = cloneProfile(profile);
    const provider = next.managed.providers.find((item) => item.provider_id === providerId);
    const model = provider?.models.find((item) => item.model_id === modelId);
    if (model) model.thinking.selected = selected;
    return next;
}

function statusTone(compatibility: string): string {
    if (compatibility === 'supported') return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300';
    if (compatibility === 'partial') return 'border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300';
    return 'border-destructive/30 bg-destructive/10 text-destructive';
}

function computeLaunchCommand(profile: AgentProfileDocument): string {
    if (profile.agent === 'opencode') {
        return 'opencode';
    }
    if (profile.agent === 'claude_code') {
        if (profile.source.scope === 'user') {
            return 'claude';
        }
        return `claude --setting-sources "" --settings "${profile.source.path.native}"`;
    }
    if (profile.agent === 'codex') {
        if (profile.default_state.is_default || profile.source.scope === 'user') {
            return 'codex';
        }
        const profileName = profile.source.profile_name || profile.display_name;
        return `codex --profile-v2 "${profileName}"`;
    }
    return profile.launch.executable || 'agent';
}

function defaultProtocol(agent: AgentKind): ProtocolCapability {
    const protocol = agent === 'claude_code' ? 'anthropic_messages' : 'openai_responses';
    return {
        native_protocol: protocol,
        upstream_protocol: protocol,
        route: agent === 'claude_code' ? 'direct' : 'adapter',
        compatibility: 'supported',
        adapter_id: null,
        adapter_version: null,
        limitations: [],
    };
}

function providerFormFrom(provider: ProviderProfile): ProviderForm {
    return {
        provider_id: provider.provider_id,
        display_name: provider.display_name,
        base_url: provider.base_url,
        credential_kind: provider.credential.kind,
        credential_reference: provider.credential.kind === 'none' ? '' : provider.credential.reference,
        credential_display: provider.credential.display,
        api_key: '',
        clear_api_key: false,
        protocol: { ...provider.protocol },
    };
}

function emptyProviderForm(agent: AgentKind, profile: AgentProfileDocument, notConfigured: string): ProviderForm {
    const protocol = profile.managed.providers[0]?.protocol || defaultProtocol(agent);
    return {
        provider_id: '',
        display_name: '',
        base_url: '',
        credential_kind: 'none',
        credential_reference: '',
        credential_display: notConfigured,
        api_key: '',
        clear_api_key: false,
        protocol: { ...protocol },
    };
}

function modelFormFrom(model: ModelProfile): ModelForm {
    return {
        model_id: model.model_id,
        display_name: model.display_name,
        enabled: model.enabled,
        supports_reasoning: model.thinking.supports_reasoning,
        supports_effort: model.thinking.supports_effort,
        selected: model.thinking.selected || '',
        options: [...model.thinking.options],
        custom_allowed: model.thinking.custom_allowed,
        variant_values: model.thinking.variant_values ? { ...model.thinking.variant_values } : null,
    };
}

function emptyModelForm(agent: AgentKind): ModelForm {
    return {
        model_id: '',
        display_name: '',
        enabled: true,
        supports_reasoning: agent !== 'claude_code',
        supports_effort: agent !== 'claude_code',
        selected: '',
        options: agent === 'codex' ? ['none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'] : [],
        custom_allowed: false,
        variant_values: null,
    };
}

function ProfileSummaryRow({ summary, selected, onClick }: { summary: AgentProfileSummary; selected: boolean; onClick: () => void }) {
    const { t } = useTranslation();
    const compatibility = summary.compatibility === 'supported'
        ? t('agentProfiles.compatibility.editable')
        : summary.compatibility === 'partial'
            ? t('agentProfiles.compatibility.partialShort')
            : t('agentProfiles.compatibility.readOnly');
    return (
        <motion.button
            type="button"
            onClick={onClick}
            aria-pressed={selected}
            whileTap={{ scale: 0.98 }}
            className={cn(
                'group relative flex w-full items-center justify-between gap-3 overflow-hidden rounded-md px-3 py-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                selected ? 'text-foreground' : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground',
            )}
        >
            {selected && (
                <motion.span
                    layoutId="agent-profile-active-profile"
                    className="absolute inset-0 rounded-md border border-primary/30 bg-primary/10 shadow-sm"
                    initial={false}
                    transition={{ type: 'spring', bounce: 0.15, duration: 0.35 }}
                />
            )}
            <span className="relative z-10 min-w-0">
                <span className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium">{summary.display_name}</span>
                    {summary.is_default && <Badge variant="outline" className="shrink-0 border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0 text-[10px] text-emerald-700 hover:bg-emerald-500/10 dark:text-emerald-300">{t('agentProfiles.common.default')}</Badge>}
                </span>
                <span className="mt-1 block truncate text-[11px] text-muted-foreground">{summary.source_path.display}</span>
            </span>
            <span className="relative z-10 flex shrink-0 items-center gap-1.5">
                <span className={cn('hidden rounded-full border px-1.5 py-0.5 text-[10px] font-medium sm:inline-flex', statusTone(summary.compatibility))}>
                    {compatibility}
                </span>
                <ChevronRight className={cn('h-4 w-4 text-muted-foreground transition-transform', selected && 'translate-x-0.5 text-primary')} />
            </span>
        </motion.button>
    );
}

export function AgentProfilesPanel() {
    const { t } = useTranslation();
    const formatError = (error: unknown) => errorText(error, t('agentProfiles.errors.desktopRuntimeRequired'));
    const [agent, setAgent] = useState<AgentKind>('opencode');
    const [targets, setTargets] = useState<RuntimeTarget[]>([]);
    const [targetId, setTargetId] = useState('');
    const [discovery, setDiscovery] = useState<AgentProfileDiscoverResult | null>(null);
    const [selectedProfileId, setSelectedProfileId] = useState('');
    const [profile, setProfile] = useState<AgentProfileDocument | null>(null);
    const [draft, setDraft] = useState<AgentProfileDocument | null>(null);
    const [notice, setNotice] = useState<PanelNotice | null>(null);
    const [loadingTargets, setLoadingTargets] = useState(true);
    const [loadingProfiles, setLoadingProfiles] = useState(false);
    const [loadingProfile, setLoadingProfile] = useState(false);
    const [busyAction, setBusyAction] = useState<'save' | 'activate' | 'launch' | 'restore' | 'crud' | null>(null);
    const [lastSave, setLastSave] = useState<AgentProfileSaveResult | null>(null);
    const [advancedOpen, setAdvancedOpen] = useState(false);
    const [advancedText, setAdvancedText] = useState('');
    const [advancedError, setAdvancedError] = useState<string | null>(null);
    const [compatOpen, setCompatOpen] = useState(false);
    const [profileDialog, setProfileDialog] = useState<ProfileDialogKind | null>(null);
    const [profileDialogName, setProfileDialogName] = useState('');
    const [replacementProfileId, setReplacementProfileId] = useState('');
    const [profileDialogError, setProfileDialogError] = useState<string | null>(null);
    const [entityDelete, setEntityDelete] = useState<EntityDelete | null>(null);
    const [providerEditor, setProviderEditor] = useState<ProviderEditor | null>(null);
    const [providerForm, setProviderForm] = useState<ProviderForm | null>(null);
    const [providerError, setProviderError] = useState<string | null>(null);
    const [modelEditor, setModelEditor] = useState<ModelEditor | null>(null);
    const [modelForm, setModelForm] = useState<ModelForm | null>(null);
    const [variantInput, setVariantInput] = useState('');
    const [modelError, setModelError] = useState<string | null>(null);
    const [modelImport, setModelImport] = useState<ModelImportState | null>(null);
    const discoveryRequest = useRef(0);
    const profileRequest = useRef(0);
    const modelImportRequest = useRef(0);
    const pendingSecrets = useRef<Record<string, string>>({});
    const pendingClears = useRef<Record<string, boolean>>({});
    const [hasPendingCredentialWrite, setHasPendingCredentialWrite] = useState(false);
    const requestContext = JSON.stringify([agent, targetId]);
    const requestContextRef = useRef(requestContext);
    requestContextRef.current = requestContext;

    const dirty = useMemo(
        () => Boolean((profile && draft && JSON.stringify(profile) !== JSON.stringify(draft)) || hasPendingCredentialWrite),
        [profile, draft, hasPendingCredentialWrite],
    );
    const selectedSummary = discovery?.profiles.find((item) => item.profile_id === selectedProfileId) || null;
    const selectedTarget = targets.find((target) => target.target_id === targetId) || null;
    const profileForEdit = draft || profile;
    const selectedCompatibilityLabel = selectedSummary?.compatibility === 'supported'
        ? t('agentProfiles.compatibility.editable')
        : selectedSummary?.compatibility === 'partial'
            ? t('agentProfiles.compatibility.partial')
            : t('agentProfiles.compatibility.readOnly');

    useEffect(() => {
        pendingSecrets.current = {};
        pendingClears.current = {};
        setHasPendingCredentialWrite(false);
    }, [agent, targetId, selectedProfileId]);

    useEffect(() => {
        if (!isReleaseFixture()) return;
        setAgent(releaseFixtureProfile.agent);
        setTargets([releaseFixtureTarget]);
        setTargetId(releaseFixtureTarget.target_id);
        setDiscovery(releaseFixtureDiscovery);
        setSelectedProfileId(releaseFixtureProfile.profile_id);
        setProfile(releaseFixtureProfile);
        setDraft(releaseFixtureProfile);
        setLoadingTargets(false);
        setLoadingProfiles(false);
        setLoadingProfile(false);
    }, []);

    useEffect(() => {
        if (isReleaseFixture()) return;
        let active = true;
        setLoadingTargets(true);
        tauriApi.v3AgentProfileRuntimeTargets()
            .then((nextTargets) => {
                if (!active) return;
                setTargets(nextTargets);
                setTargetId((current) => current || nextTargets[0]?.target_id || '');
                if (nextTargets.length === 0) setNotice({ kind: 'info', text: t('agentProfiles.notices.noTargets') });
            })
            .catch((error) => active && setNotice({ kind: 'error', text: formatError(error) }))
            .finally(() => active && setLoadingTargets(false));
        return () => { active = false; };
    }, []);

    useEffect(() => {
        if (isReleaseFixture()) return;
        const requestId = ++discoveryRequest.current;
        const requestContextSnapshot = requestContextRef.current;
        // A new Agent/runtime context invalidates any read still in flight.
        profileRequest.current += 1;
        setLoadingProfile(false);
        if (!targetId) return;
        setLoadingProfiles(true);
        setDiscovery(null);
        setProfile(null);
        setDraft(null);
        setSelectedProfileId('');
        setLastSave(null);
        tauriApi.v3AgentProfileDiscover({ agent, runtime_target_id: targetId })
            .then((result) => {
                if (requestId !== discoveryRequest.current || requestContextRef.current !== requestContextSnapshot) return;
                setDiscovery(result);
                setSelectedProfileId(result.default_profile_id || result.profiles[0]?.profile_id || '');
                const discoverErrors = result.errors ?? [];
                if (discoverErrors.length > 0) {
                    const text = discoverErrors
                        .map((entry) => {
                            const structured = structuredConfigError(entry);
                            return structured || entry.code || JSON.stringify(entry);
                        })
                        .join('; ');
                    setNotice({ kind: discoverErrors.length > 0 && result.profiles.length > 0 ? 'info' : 'error', text });
                } else {
                    setNotice(null);
                }
            })
            .catch((error) => requestId === discoveryRequest.current && requestContextRef.current === requestContextSnapshot && setNotice({ kind: 'error', text: formatError(error) }))
            .finally(() => requestId === discoveryRequest.current && requestContextRef.current === requestContextSnapshot && setLoadingProfiles(false));
    }, [agent, targetId]);

    const discoveryMatchesContext = Boolean(
        discovery
        && discovery.agent === agent
        && discovery.runtime_targets.some((target) => target.target_id === targetId)
        && discovery.profiles.some((item) => item.profile_id === selectedProfileId),
    );

    useEffect(() => {
        if (isReleaseFixture()) return;
        if (!targetId || !selectedProfileId || !discoveryMatchesContext) return;
        const requestId = ++profileRequest.current;
        const requestContextSnapshot = requestContextRef.current;
        setLoadingProfile(true);
        tauriApi.v3AgentProfileRead({ agent, runtime_target_id: targetId, profile_id: selectedProfileId })
            .then((result) => {
                if (requestId !== profileRequest.current || requestContextRef.current !== requestContextSnapshot) return;
                setProfile(result.profile);
                setDraft(cloneProfile(result.profile));
                setAdvancedOpen(false);
                setAdvancedText('');
                setAdvancedError(null);
            })
            .catch((error) => requestId === profileRequest.current && requestContextRef.current === requestContextSnapshot && setNotice({ kind: 'error', text: formatError(error) }))
            .finally(() => requestId === profileRequest.current && requestContextRef.current === requestContextSnapshot && setLoadingProfile(false));
        return () => {
            if (requestId === profileRequest.current) profileRequest.current += 1;
        };
    }, [agent, discoveryMatchesContext, discovery, selectedProfileId, targetId]);

    const refresh = () => {
        if (!targetId) return;
        setSelectedProfileId('');
        setDiscovery(null);
        setProfile(null);
        setDraft(null);
        setLastSave(null);
        setNotice(null);
        profileRequest.current += 1;
        setLoadingProfile(false);
        const requestId = ++discoveryRequest.current;
        const requestContextSnapshot = requestContextRef.current;
        setLoadingProfiles(true);
        tauriApi.v3AgentProfileDiscover({ agent, runtime_target_id: targetId })
            .then((result) => {
                if (requestId !== discoveryRequest.current || requestContextRef.current !== requestContextSnapshot) return;
                setDiscovery(result);
                setSelectedProfileId(result.default_profile_id || result.profiles[0]?.profile_id || '');
            })
            .catch((error) => requestId === discoveryRequest.current && requestContextRef.current === requestContextSnapshot && setNotice({ kind: 'error', text: formatError(error) }))
            .finally(() => requestId === discoveryRequest.current && requestContextRef.current === requestContextSnapshot && setLoadingProfiles(false));
    };

    const saveDraft = async () => {
        if (!draft || !profile || !targetId) return;
        setBusyAction('save');
        setNotice(null);
        try {
            const request: V3AgentProfileSaveRequest = {
                agent,
                runtime_target_id: targetId,
                profile_id: draft.profile_id,
                expected_revision: profile.revision.revision,
                profile: attachCredentialWrite(draft, pendingSecrets.current, pendingClears.current),
            };
            const result = await tauriApi.v3AgentProfileSave(request);
            pendingSecrets.current = {};
            pendingClears.current = {};
            setHasPendingCredentialWrite(false);
            setProfile(result.profile);
            setDraft(cloneProfile(result.profile));
            setLastSave(result);
            setNotice({ kind: 'success', text: t('agentProfiles.notices.saved') });
        } catch (error) {
            setNotice({ kind: 'error', text: t('agentProfiles.errors.saveFailed', { message: formatError(error) }) });
        } finally {
            setBusyAction(null);
        }
    };

    const activateDraft = async () => {
        if (!profile || !targetId) return;
        setBusyAction('activate');
        setNotice(null);
        try {
            const result = await tauriApi.v3AgentProfileActivate({
                agent,
                runtime_target_id: targetId,
                profile_id: profile.profile_id,
                expected_revision: profile.revision.revision,
            });
            setProfile(result.profile);
            setDraft(cloneProfile(result.profile));
            setLastSave(result);
            setNotice({ kind: 'success', text: t('agentProfiles.notices.defaultChanged') });
            refresh();
        } catch (error) {
            setNotice({ kind: 'error', text: t('agentProfiles.errors.activateFailed', { message: formatError(error) }) });
        } finally {
            setBusyAction(null);
        }
    };

    const prepareLaunch = async () => {
        if (!profile || !targetId) return;
        setBusyAction('launch');
        setNotice(null);
        try {
            const result = await tauriApi.v3AgentProfileLaunch({
                agent,
                runtime_target_id: targetId,
                profile_id: profile.profile_id,
                launch_mode: 'temporary',
            });
            setLastSave(result);
            setNotice({ kind: 'success', text: t('agentProfiles.notices.launched') });
        } catch (error) {
            setNotice({ kind: 'error', text: formatError(error) });
        } finally {
            setBusyAction(null);
        }
    };

    const copyLaunchCommand = async () => {
        if (!profileForEdit) return;
        const command = computeLaunchCommand(profileForEdit);
        try {
            await navigator.clipboard.writeText(command);
            setNotice({ kind: 'info', text: t('agentProfiles.notices.commandCopied', { command }) });
        } catch {
            setNotice({ kind: 'info', text: t('agentProfiles.notices.commandCopied', { command }) });
        }
    };

    const restoreLastSave = async () => {
        if (!profile || !lastSave?.backup_path || !targetId) return;
        setBusyAction('restore');
        setNotice(null);
        try {
            const result = await tauriApi.v3AgentProfileRestore({
                agent,
                runtime_target_id: targetId,
                profile_id: profile.profile_id,
                expected_revision: profile.revision.revision,
                backup_path: lastSave.backup_path.native,
            });
            setProfile(result.profile);
            setDraft(cloneProfile(result.profile));
            setLastSave(result);
            setNotice({ kind: 'success', text: t('agentProfiles.notices.restored') });
        } catch (error) {
            setNotice({ kind: 'error', text: t('agentProfiles.errors.restoreFailed', { message: formatError(error) }) });
        } finally {
            setBusyAction(null);
        }
    };

    const openProfileDialog = (kind: ProfileDialogKind) => {
        setProfileDialog(kind);
        setProfileDialogError(null);
        setProfileDialogName(kind === 'rename' ? profileForEdit?.display_name || '' : '');
        setReplacementProfileId('');
    };

    const closeProfileDialog = () => {
        if (busyAction === 'crud') return;
        setProfileDialog(null);
        setProfileDialogError(null);
    };

    const submitProfileDialog = async () => {
        if (!profileDialog || !targetId) return;
        if (profileDialog !== 'delete' && !profileDialogName.trim()) {
            setProfileDialogError(t('agentProfiles.errors.profileNameRequired'));
            return;
        }
        if (profileDialog === 'delete' && !profileForEdit) return;
        if (profileDialog === 'delete' && selectedSummary?.is_default && !replacementProfileId) {
            setProfileDialogError(t('agentProfiles.errors.replacementRequired'));
            return;
        }
        setBusyAction('crud');
        setProfileDialogError(null);
        setNotice(null);
        try {
            if (profileDialog === 'create') {
                await tauriApi.v3AgentProfileCreate({ agent, runtime_target_id: targetId, profile_name: profileDialogName.trim() });
                setNotice({ kind: 'success', text: t('agentProfiles.notices.profileCreated', { name: profileDialogName.trim() }) });
            } else if (profileDialog === 'clone') {
                await tauriApi.v3AgentProfileClone({ agent, runtime_target_id: targetId, source_profile_id: selectedProfileId, profile_name: profileDialogName.trim() });
                setNotice({ kind: 'success', text: t('agentProfiles.notices.profileCloned', { name: profileDialogName.trim() }) });
            } else if (profileDialog === 'rename') {
                await tauriApi.v3AgentProfileRename({ agent, runtime_target_id: targetId, profile_id: selectedProfileId, new_profile_name: profileDialogName.trim() });
                setNotice({ kind: 'success', text: t('agentProfiles.notices.profileRenamed', { name: profileDialogName.trim() }) });
            } else {
                await tauriApi.v3AgentProfileDelete({
                    agent,
                    runtime_target_id: targetId,
                    profile_id: selectedProfileId,
                    replacement_profile_id: replacementProfileId || undefined,
                });
                setNotice({ kind: 'success', text: t('agentProfiles.notices.profileDeleted') });
            }
            setProfileDialog(null);
            refresh();
        } catch (error) {
            setProfileDialogError(formatError(error));
        } finally {
            setBusyAction(null);
        }
    };

    const openProviderEditor = (provider?: ProviderProfile) => {
        if (!profileForEdit) return;
        if (!provider && agent === 'claude_code') {
            setNotice({ kind: 'info', text: t('agentProfiles.notices.claudeSingleProvider') });
            return;
        }
        setProviderEditor(provider ? { mode: 'edit', providerId: provider.provider_id } : { mode: 'create' });
        setProviderForm(provider ? providerFormFrom(provider) : emptyProviderForm(agent, profileForEdit, t('agentProfiles.common.notConfigured')));
        setProviderError(null);
    };

    const closeProviderEditor = () => {
        setProviderEditor(null);
        setProviderForm(null);
        setProviderError(null);
    };

    const submitProviderEditor = () => {
        if (!providerEditor || !providerForm || !profileForEdit) return;
        const providerId = providerForm.provider_id.trim();
        const displayName = providerForm.display_name.trim();
        if (!providerId || !/^[a-zA-Z0-9._-]+$/.test(providerId)) {
            setProviderError(t('agentProfiles.errors.providerIdInvalid'));
            return;
        }
        if (!displayName) {
            setProviderError(t('agentProfiles.errors.providerNameRequired'));
            return;
        }
        if (providerForm.base_url.trim()) {
            try { new URL(providerForm.base_url.trim()); } catch { setProviderError(t('agentProfiles.errors.baseUrlInvalid')); return; }
        }
        const duplicate = profileForEdit.managed.providers.some((provider) => provider.provider_id === providerId && provider.provider_id !== providerEditor.providerId);
        if (duplicate) {
            setProviderError(t('agentProfiles.errors.providerExists'));
            return;
        }
        const trimmedKey = providerForm.api_key.trim();
        if (providerForm.clear_api_key) {
            delete pendingSecrets.current[providerId];
            pendingClears.current[providerId] = true;
            setHasPendingCredentialWrite(true);
        } else if (trimmedKey) {
            pendingSecrets.current[providerId] = trimmedKey;
            delete pendingClears.current[providerId];
            setHasPendingCredentialWrite(true);
        }
        const credential: CredentialReference = agent === 'claude_code'
            ? {
                kind: 'env',
                reference: 'ANTHROPIC_AUTH_TOKEN',
                display: providerForm.clear_api_key
                    ? t('agentProfiles.common.notConfigured')
                    : (trimmedKey
                        ? t('agentProfiles.credentials.configured')
                        : (providerForm.credential_display.trim() || t('agentProfiles.common.notConfigured'))),
                secret_state: providerForm.clear_api_key
                    ? 'missing'
                    : ((trimmedKey || providerForm.credential_display.trim()) ? 'configured' : 'missing'),
                persisted_in_config: false,
            }
            : (providerForm.clear_api_key || trimmedKey)
                ? {
                    kind: 'unknown',
                    reference: 'credential:configured',
                    display: providerForm.clear_api_key
                        ? t('agentProfiles.common.notConfigured')
                        : t('agentProfiles.credentials.configured'),
                    secret_state: providerForm.clear_api_key ? 'missing' : 'configured',
                    persisted_in_config: false,
                }
                : {
                    kind: providerForm.credential_kind,
                    reference: providerForm.credential_kind === 'none' ? 'none' : providerForm.credential_reference.trim(),
                    display: providerForm.credential_display.trim() || (providerForm.credential_kind === 'none' ? t('agentProfiles.common.notConfigured') : providerForm.credential_reference.trim()),
                    secret_state: providerForm.credential_kind === 'none' ? 'missing' : 'configured',
                    persisted_in_config: false,
                };
        if (agent !== 'claude_code' && credential.kind === 'env' && !/^[A-Za-z_][A-Za-z0-9_]*$/.test(credential.reference)) {
            setProviderError(t('agentProfiles.errors.credentialReferenceInvalid'));
            return;
        }
        const nextProvider: ProviderProfile = {
            provider_id: providerId,
            display_name: displayName,
            base_url: providerForm.base_url.trim(),
            credential,
            protocol: providerForm.protocol,
            models: profileForEdit.managed.providers.find((item) => item.provider_id === providerEditor.providerId)?.models || [],
        };
        setDraft((current) => {
            if (!current) return current;
            const next = cloneProfile(current);
            const index = next.managed.providers.findIndex((provider) => provider.provider_id === providerEditor.providerId);
            if (index >= 0) next.managed.providers[index] = nextProvider;
            else next.managed.providers.push(nextProvider);
            if (!next.managed.default_provider_id) next.managed.default_provider_id = providerId;
            return next;
        });
        closeProviderEditor();
    };

    const openModelEditor = (provider: ProviderProfile, model?: ModelProfile) => {
        setModelEditor(model ? { mode: 'edit', providerId: provider.provider_id, modelId: model.model_id } : { mode: 'create', providerId: provider.provider_id });
        setModelForm(model ? modelFormFrom(model) : emptyModelForm(agent));
        setVariantInput('');
        setModelError(null);
    };

    const closeModelEditor = () => {
        setModelEditor(null);
        setModelForm(null);
        setVariantInput('');
        setModelError(null);
    };

    const submitModelEditor = () => {
        if (!modelEditor || !modelForm || !profileForEdit) return;
        const modelId = modelForm.model_id.trim();
        const displayName = modelForm.display_name.trim();
        if (!modelId || !displayName) {
            setModelError(t('agentProfiles.errors.modelFieldsRequired'));
            return;
        }
        const provider = profileForEdit.managed.providers.find((item) => item.provider_id === modelEditor.providerId);
        if (!provider) return;
        if (provider.models.some((model) => model.model_id === modelId && model.model_id !== modelEditor.modelId)) {
            setModelError(t('agentProfiles.errors.modelExists'));
            return;
        }
        const options = Array.from(new Set(modelForm.options.map((option) => option.trim()).filter(Boolean)));
        const selected = modelForm.selected && options.includes(modelForm.selected) ? modelForm.selected : options[0] || null;
        const previousModel = modelEditor.modelId
            ? provider.models.find((model) => model.model_id === modelEditor.modelId)
            : undefined;
        const previousOptions = previousModel?.thinking.options || [];
        const variantValuesChanged = agent === 'opencode'
            && (previousModel === undefined || JSON.stringify(previousOptions) !== JSON.stringify(options));
        const variantValues = modelForm.variant_values
            ? Object.fromEntries(options.map((option) => [option, modelForm.variant_values?.[option] ?? {}]))
            : null;
        const nextModel: ModelProfile = {
            model_id: modelId,
            display_name: displayName,
            enabled: modelForm.enabled,
            thinking: {
                supports_reasoning: modelForm.supports_reasoning,
                supports_effort: modelForm.supports_effort,
                selected,
                options,
                custom_allowed: modelForm.custom_allowed,
                variant_values: variantValues,
                variant_values_changed: variantValuesChanged,
            },
        };
        setDraft((current) => {
            if (!current) return current;
            const next = cloneProfile(current);
            const target = next.managed.providers.find((item) => item.provider_id === modelEditor.providerId);
            if (!target) return current;
            const index = target.models.findIndex((model) => model.model_id === modelEditor.modelId);
            if (index >= 0) target.models[index] = nextModel;
            else target.models.push(nextModel);
            const wasDefault = modelEditor.modelId ? modelIsDefault(current, modelEditor.providerId, modelEditor.modelId) : !current.managed.default_model_id;
            const replacementProvider = next.managed.providers.find((item) => item.models.some((model) => model.enabled));
            const replacement = replacementProvider?.models.find((model) => model.enabled);
            if (current.agent !== 'opencode') {
                next.managed.default_provider_id = nextModel.enabled ? modelEditor.providerId : null;
                next.managed.default_model_id = nextModel.enabled ? modelId : null;
            } else if (nextModel.enabled && (wasDefault || !next.managed.default_model_id)) {
                next.managed.default_provider_id = modelEditor.providerId;
                next.managed.default_model_id = profileModelKey(modelEditor.providerId, modelId);
            } else if (!nextModel.enabled && wasDefault) {
                next.managed.default_provider_id = replacementProvider?.provider_id || null;
                next.managed.default_model_id = replacement && replacementProvider ? profileModelKey(replacementProvider.provider_id, replacement.model_id) : null;
            }
            return next;
        });
        closeModelEditor();
    };

    const closeModelImport = () => {
        modelImportRequest.current += 1;
        setModelImport(null);
    };

    const openModelImport = async (provider: ProviderProfile) => {
        if (!provider.base_url.trim()) {
            setNotice({ kind: 'error', text: t('agentProfiles.errors.detectBaseUrlRequired') });
            return;
        }
        if (isReleaseFixture()) {
            const existing = new Set(provider.models.map((model) => model.model_id));
            setModelImport({
                providerId: provider.provider_id,
                loading: false,
                error: null,
                endpoint: `${provider.base_url.replace(/\/$/, '')}/models`,
                items: releaseFixtureUpstreamModels.map((model) => ({
                    model_id: model.model_id,
                    display_name: model.display_name,
                    imported: Boolean(model.imported) || existing.has(model.model_id),
                })),
                selected: releaseFixtureUpstreamModels.filter((model) => !model.imported && !existing.has(model.model_id)).slice(0, 3).map((model) => model.model_id),
                filter: '',
            });
            return;
        }
        const requestId = ++modelImportRequest.current;
        setModelImport({
            providerId: provider.provider_id,
            loading: true,
            error: null,
            endpoint: null,
            items: [],
            selected: [],
            filter: '',
        });
        try {
            const pendingKey = pendingSecrets.current[provider.provider_id]?.trim();
            const result = await tauriApi.v3AgentProfileListUpstreamModels({
                agent,
                runtime_target_id: targetId,
                profile_id: selectedProfileId,
                provider_id: provider.provider_id,
                base_url: provider.base_url,
                protocol: listModelsProtocol(agent, provider),
                ...(pendingKey ? { api_key: pendingKey } : {}),
            });
            if (requestId !== modelImportRequest.current) return;
            const existing = new Set(provider.models.map((model) => model.model_id));
            const items = result.models.map((model) => ({
                model_id: model.model_id,
                display_name: model.display_name || model.model_id,
                imported: existing.has(model.model_id),
            }));
            setModelImport({
                providerId: provider.provider_id,
                loading: false,
                error: null,
                endpoint: result.endpoint,
                items,
                selected: [],
                filter: '',
            });
        } catch (error) {
            if (requestId !== modelImportRequest.current) return;
            setModelImport({
                providerId: provider.provider_id,
                loading: false,
                error: detectErrorText(error, t('agentProfiles.errors.desktopRuntimeRequired'), (key) => t(key)),
                endpoint: null,
                items: [],
                selected: [],
                filter: '',
            });
        }
    };

    useDevScreenshotCue((command) => {
        if (command === 'close') {
            closeProviderEditor();
            closeModelEditor();
            closeModelImport();
            return;
        }
        const provider = profileForEdit?.managed.providers[0];
        if (!provider) return;
        if (command === 'provider') openProviderEditor(provider);
        if (command === 'import') void openModelImport(provider);
    });

    const submitModelImport = () => {
        if (!modelImport) return;
        const selected = new Set(modelImport.selected);
        const imported = modelImport.items.filter((item) => selected.has(item.model_id) && !item.imported);
        if (imported.length === 0) {
            closeModelImport();
            return;
        }
        setDraft((current) => current ? importUpstreamModelsIntoDraft(current, modelImport.providerId, imported) : current);
        setNotice({ kind: 'success', text: t('agentProfiles.notices.modelsImported', { count: imported.length }) });
        closeModelImport();
    };

    const openEntityDelete = (entity: EntityDelete) => setEntityDelete(entity);

    const submitEntityDelete = () => {
        if (!entityDelete) return;
        if (entityDelete.kind === 'provider' && agent === 'claude_code') {
            setNotice({ kind: 'info', text: t('agentProfiles.notices.claudeCannotDeleteProvider') });
            setEntityDelete(null);
            return;
        }
        setDraft((current) => {
            if (!current) return current;
            const next = cloneProfile(current);
            if (entityDelete.kind === 'provider') {
                next.managed.providers = next.managed.providers.filter((provider) => provider.provider_id !== entityDelete.providerId);
                if (next.managed.default_provider_id === entityDelete.providerId) {
                    const replacement = next.managed.providers[0];
                    next.managed.default_provider_id = replacement?.provider_id || null;
                    const model = replacement?.models[0];
                    next.managed.default_model_id = model ? (next.agent === 'opencode' ? profileModelKey(replacement!.provider_id, model.model_id) : model.model_id) : null;
                }
            } else {
                const provider = next.managed.providers.find((item) => item.provider_id === entityDelete.providerId);
                if (provider) provider.models = provider.models.filter((model) => model.model_id !== entityDelete.modelId);
                if (entityDelete.modelId && modelIsDefault(current, entityDelete.providerId, entityDelete.modelId)) {
                    const replacement = provider?.models[0] || next.managed.providers.find((item) => item.models.length > 0)?.models[0];
                    const replacementProvider = next.managed.providers.find((item) => item.models.includes(replacement as ModelProfile));
                    next.managed.default_provider_id = replacementProvider?.provider_id || null;
                    next.managed.default_model_id = replacement && replacementProvider ? (next.agent === 'opencode' ? profileModelKey(replacementProvider.provider_id, replacement.model_id) : replacement.model_id) : null;
                }
            }
            return next;
        });
        setEntityDelete(null);
    };

    const toggleAdvanced = () => {
        setAdvancedOpen((current) => {
            const next = !current;
            if (next && profileForEdit) {
                setAdvancedText(JSON.stringify(profileForEdit, null, 2));
                setAdvancedError(null);
            }
            return next;
        });
    };

    const applyAdvanced = () => {
        if (!profile || !profileForEdit) return;
        try {
            const parsed = JSON.parse(advancedText) as AgentProfileDocument;
            if (parsed.profile_id !== profile.profile_id || parsed.runtime_target_id !== profile.runtime_target_id) {
                throw new Error(t('agentProfiles.errors.advancedIdentityImmutable'));
            }
            if (!parsed.managed || !Array.isArray(parsed.managed.providers)) throw new Error(t('agentProfiles.errors.advancedProvidersMissing'));
            setDraft(parsed);
            setAdvancedError(null);
            setNotice({ kind: 'info', text: t('agentProfiles.notices.advancedApplied') });
        } catch (error) {
            setAdvancedError(formatError(error));
        }
    };

    const canCreateProfile = agent !== 'opencode';
    const canCreateProvider = agent !== 'claude_code';
    const profileDialogTitle = profileDialog ? t(`agentProfiles.dialogs.profile.titles.${profileDialog}`) : '';
    const profileDialogDescription = profileDialog === 'delete'
        ? t('agentProfiles.dialogs.profile.deleteDescription', { name: profileForEdit?.display_name || '' })
        : t('agentProfiles.dialogs.profile.description');

    return (
        <section className={cn('space-y-5 animate-in fade-in slide-in-from-bottom-2 duration-200 ease-out', interactionGroupClass)} aria-labelledby="agent-profiles-title">
            <header className="flex flex-col gap-3 pb-1 sm:flex-row sm:items-end sm:justify-between">
                <div>
                    <h1 id="agent-profiles-title" className="text-2xl font-semibold tracking-tight">{t('agentProfiles.title')}</h1>
                    <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">{t('agentProfiles.subtitle')}</p>
                </div>
                <div className="flex items-center gap-2 pb-1 text-[11px] text-muted-foreground">
                    <KeyRound className="h-3.5 w-3.5" />{t('agentProfiles.securityNoteClaude')}
                </div>
            </header>

            <nav className="flex flex-wrap items-center gap-1" role="tablist" aria-label={t('agentProfiles.selectAgent')}>
                {AGENTS.map((item) => {
                    const Icon = item.icon;
                    const selected = agent === item.id;
                    return (
                        <motion.button
                            key={item.id}
                            type="button"
                            role="tab"
                            aria-selected={selected}
                            title={t(item.descriptionKey)}
                            onClick={() => setAgent(item.id)}
                            whileTap={{ scale: 0.97 }}
                            className={cn('relative flex items-center gap-2 px-3 py-2 text-left text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring', selected ? 'font-medium text-foreground' : 'text-muted-foreground hover:text-foreground')}
                        >
                            <Icon className="h-4 w-4" />
                            <span>{item.label}</span>
                            {selected && (
                                <motion.span
                                    layoutId="agent-profile-active-agent-indicator"
                                    className="absolute inset-x-2 -bottom-px h-0.5 rounded-full bg-foreground"
                                    initial={false}
                                    transition={{ type: 'spring', bounce: 0.15, duration: 0.35 }}
                                />
                            )}
                        </motion.button>
                    );
                })}
            </nav>

            <div className="flex flex-col gap-3 pb-2 sm:flex-row sm:items-end sm:justify-between">
                <div className="min-w-0 flex-1">
                    <Label htmlFor="agent-runtime-target" className="mb-1.5 flex items-center gap-2 text-xs font-medium text-muted-foreground"><Server className="h-3.5 w-3.5" />{t('agentProfiles.runtimeTarget')}</Label>
                    <select id="agent-runtime-target" aria-label={t('agentProfiles.selectRuntimeTarget')} value={targetId} onChange={(event) => setTargetId(event.target.value)} disabled={loadingTargets} className={fieldClass}>
                        <option value="">{loadingTargets ? t('agentProfiles.discoveringRuntimes') : t('agentProfiles.selectRuntime')}</option>
                        {targets.map((target) => <option key={target.target_id} value={target.target_id}>{target.display_name} · {target.home_path.display}</option>)}
                    </select>
                </div>
                <div className="flex items-center gap-2">
                    {selectedTarget && <span className="hidden max-w-xs items-center gap-1.5 truncate text-xs text-muted-foreground lg:flex">{selectedTarget.kind === 'wsl' ? <Cloud className="h-3.5 w-3.5 shrink-0" /> : <Laptop className="h-3.5 w-3.5 shrink-0" />}{selectedTarget.home_path.native}</span>}
                    <Button type="button" variant="outline" onClick={refresh} disabled={!targetId || loadingProfiles} aria-label={t('agentProfiles.refreshProfiles')}><RefreshCw className={cn('mr-2 h-4 w-4', loadingProfiles && 'animate-spin')} />{t('agentProfiles.common.refresh')}</Button>
                </div>
            </div>

            {notice && <div className={cn('flex items-start gap-2 rounded-md border px-3 py-2 text-xs shadow-sm', notice.kind === 'success' && 'border-emerald-500/25 bg-emerald-500/5 text-emerald-700 dark:text-emerald-300', notice.kind === 'error' && 'border-destructive/25 bg-destructive/5 text-destructive', notice.kind === 'info' && 'border-primary/20 bg-primary/5 text-primary')} role={notice.kind === 'error' ? 'alert' : 'status'}>{notice.kind === 'error' ? <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" /> : <Check className="mt-0.5 h-3.5 w-3.5 shrink-0" />}<span className="leading-5">{notice.text}</span></div>}

            {!targetId || (!loadingProfiles && discovery && discovery.profiles.length === 0) ? (
                <div className="py-16 text-center">
                    <FileCode2 className="mx-auto h-8 w-8 text-primary/70" />
                    <h2 className="mt-4 text-lg font-semibold">{t(targetId ? 'agentProfiles.empty.noProfilesTitle' : 'agentProfiles.empty.selectRuntimeTitle')}</h2>
                    <p className="mx-auto mt-2 max-w-md text-sm leading-6 text-muted-foreground">{targetId ? t('agentProfiles.empty.noProfilesDescription', { agent: agentLabel(agent) }) : t('agentProfiles.empty.selectRuntimeDescription')}</p>
                </div>
            ) : loadingProfiles || loadingProfile ? (
                <div className="flex items-center justify-center gap-3 py-20 text-sm text-muted-foreground"><Loader2 className="h-5 w-5 animate-spin" />{t('agentProfiles.loadingProfiles')}</div>
            ) : discovery && profileForEdit ? (
                <div className="grid gap-4 xl:grid-cols-[15rem_minmax(0,1fr)]">
                    <aside className="min-w-0" aria-label={t('agentProfiles.profiles.listLabel')}>
                        <div className="flex items-center justify-between gap-2 pb-2">
                            <div className="flex min-w-0 items-center gap-2"><FileCode2 className="h-4 w-4 shrink-0 text-muted-foreground" /><div><h2 className="text-sm font-semibold">{t('agentProfiles.profiles.title')}</h2><p className="mt-0.5 text-[11px] text-muted-foreground">{t('agentProfiles.profiles.count', { count: discovery.profiles.length })}</p></div></div>
                            <div className="flex items-center gap-1">
                                <Button type="button" variant="ghost" size="icon" className="h-8 w-8" onClick={() => openProfileDialog('create')} disabled={!canCreateProfile || busyAction !== null} title={t(canCreateProfile ? 'agentProfiles.profiles.create' : 'agentProfiles.profiles.openCodeSingle')} aria-label={t('agentProfiles.profiles.create')}><Plus className="h-4 w-4" /></Button>
                                <Button type="button" variant="ghost" size="icon" className="h-8 w-8" onClick={() => openProfileDialog('clone')} disabled={!canCreateProfile || !selectedProfileId || busyAction !== null} title={t('agentProfiles.profiles.cloneCurrent')} aria-label={t('agentProfiles.profiles.cloneCurrent')}><Copy className="h-4 w-4" /></Button>
                            </div>
                        </div>
                        <div className="space-y-1">
                            {discovery.profiles.map((summary) => <ProfileSummaryRow key={summary.profile_id} summary={summary} selected={summary.profile_id === selectedProfileId} onClick={() => setSelectedProfileId(summary.profile_id)} />)}
                        </div>
                        <div className="flex items-center justify-end gap-1 pt-2">
                            <Button type="button" variant="ghost" size="sm" onClick={() => openProfileDialog('rename')} disabled={!canCreateProfile || !selectedProfileId || busyAction !== null}><Pencil className="mr-1.5 h-3.5 w-3.5" />{t('agentProfiles.common.rename')}</Button>
                            <Button type="button" variant="ghost" size="sm" className="text-destructive hover:text-destructive" onClick={() => openProfileDialog('delete')} disabled={!canCreateProfile || !selectedProfileId || busyAction !== null}><Trash2 className="mr-1.5 h-3.5 w-3.5" />{t('agentProfiles.common.delete')}</Button>
                        </div>
                    </aside>

                    <motion.article key={`${agent}:${selectedProfileId}`} initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.18, ease: 'easeOut' }} className="min-w-0 rounded-xl bg-muted/[0.18] p-3 shadow-sm ring-1 ring-border/25 sm:p-4">
                        <header className="flex flex-col gap-4 px-1 pb-3 pt-1 sm:px-2 xl:flex-row xl:items-start xl:justify-between">
                            <div className="min-w-0">
                                <div className="flex flex-wrap items-center gap-2">
                                    <h2 className="text-xl font-semibold">{profileForEdit.display_name}</h2>
                                    {selectedSummary?.is_default && <Badge variant="outline" className="gap-1 border-emerald-500/30 bg-emerald-500/10 text-emerald-700 hover:bg-emerald-500/10 dark:text-emerald-300"><Check className="h-3 w-3" />{t('agentProfiles.common.defaultProfile')}</Badge>}
                                    <span className={cn('rounded-full border px-2 py-0.5 text-[10px] font-medium', statusTone(selectedSummary?.compatibility || 'unknown'))}>{selectedCompatibilityLabel}</span>
                                    {dirty && <Badge variant="outline" className="border-amber-500/40 text-amber-600">{t('agentProfiles.common.unsaved')}</Badge>}
                                </div>
                                <div className="mt-2 flex min-w-0 items-center gap-1 text-xs text-muted-foreground"><FileCode2 className="h-3.5 w-3.5 shrink-0" /><span className="truncate">{profileForEdit.source.path.display}</span></div>
                            </div>
                            <div className="flex flex-wrap gap-2">
                                <Button type="button" variant="outline" onClick={activateDraft} disabled={!profile || busyAction !== null || selectedSummary?.is_default}>{busyAction === 'activate' ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Sparkles className="mr-2 h-4 w-4" />}{t('agentProfiles.actions.setDefault')}</Button>
                                <Button type="button" variant="outline" onClick={copyLaunchCommand} disabled={!profileForEdit || busyAction !== null} title={t('agentProfiles.actions.copyLaunchCommand')} aria-label={t('agentProfiles.actions.copyLaunchCommand')}><Copy className="mr-2 h-4 w-4" />{t('agentProfiles.actions.copyLaunchCommand')}</Button>
                                <Button type="button" variant="outline" onClick={prepareLaunch} disabled={!profile || busyAction !== null}>{busyAction === 'launch' ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Terminal className="mr-2 h-4 w-4" />}{t('agentProfiles.actions.launchOnce')}</Button>
                                <Button type="button" onClick={saveDraft} disabled={!dirty || busyAction !== null}>{busyAction === 'save' ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Save className="mr-2 h-4 w-4" />}{t('agentProfiles.common.save')}</Button>
                            </div>
                        </header>

                        <div className="flex flex-wrap gap-x-6 gap-y-2 px-1 pb-1 pt-2 text-xs text-muted-foreground sm:px-2">
                            <span className="flex items-center gap-1.5"><Settings2 className="h-3.5 w-3.5 text-primary" />{t(profileForEdit.protocol.route === 'adapter' ? 'agentProfiles.route.adapter' : profileForEdit.protocol.route === 'direct' ? 'agentProfiles.route.direct' : 'agentProfiles.route.pending')}</span>
                            <span className="flex items-center gap-1.5"><KeyRound className="h-3.5 w-3.5 text-primary" />{profileForEdit.managed.providers[0]?.credential.display || t('agentProfiles.credentials.notConfigured')}</span>
                            <span>{t('agentProfiles.models.count', { count: profileForEdit.managed.providers.reduce((count, provider) => count + provider.models.length, 0) })}</span>
                        </div>

                        <section className="mt-5" aria-labelledby="providers-title">
                            <div className="flex items-end justify-between gap-3 px-1 pb-3 sm:px-2"><div><h3 id="providers-title" className="text-base font-semibold">{t('agentProfiles.providers.title')}</h3><p className="mt-1 text-xs text-muted-foreground">{t('agentProfiles.providers.description')}</p></div><Button type="button" variant="outline" size="sm" onClick={() => openProviderEditor()} disabled={!canCreateProvider || busyAction !== null}><Plus className="mr-1.5 h-3.5 w-3.5" />{t('agentProfiles.providers.add')}</Button></div>
                            {profileForEdit.managed.providers.length === 0 && <div className="py-8 text-sm text-muted-foreground">{t('agentProfiles.providers.empty')}</div>}
                            <div className="space-y-3">
                                {profileForEdit.managed.providers.map((provider) => (
                                    <div key={provider.provider_id} className="rounded-lg bg-background/65 px-4 py-4 shadow-sm ring-1 ring-border/25 sm:px-5">
                                        <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                                            <div className="min-w-0"><div className="flex items-center gap-2"><Cloud className="h-4 w-4 text-primary" /><div className="truncate font-medium">{provider.display_name}</div><span className="font-mono text-[11px] text-muted-foreground">{provider.provider_id}</span></div><div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground"><KeyRound className="h-3.5 w-3.5" />{provider.credential.display}</div></div>
                                            <div className="flex shrink-0 items-center gap-1"><Button type="button" variant="ghost" size="sm" onClick={() => openProviderEditor(provider)}><Pencil className="mr-1.5 h-3.5 w-3.5" />{t('agentProfiles.providers.edit')}</Button><Button type="button" variant="ghost" size="icon" className="h-8 w-8 text-destructive hover:text-destructive" onClick={() => openEntityDelete({ kind: 'provider', providerId: provider.provider_id, label: provider.display_name })} disabled={agent === 'claude_code'} title={t('agentProfiles.providers.delete')} aria-label={t('agentProfiles.providers.deleteNamed', { name: provider.display_name })}><Trash2 className="h-3.5 w-3.5" /></Button></div>
                                        </div>
                                        <div className="mt-4 flex flex-wrap items-center gap-3"><span className="text-xs text-muted-foreground">Base URL</span><span className="min-w-0 truncate font-mono text-xs">{provider.base_url || t('agentProfiles.common.notConfigured')}</span><div className="ml-auto flex flex-wrap items-center gap-1"><Button type="button" variant="ghost" size="sm" onClick={() => openModelImport(provider)} disabled={!provider.base_url.trim() || busyAction !== null || modelImport?.loading === true} title={t('agentProfiles.models.detectNamed', { name: provider.display_name })} aria-label={t('agentProfiles.models.detectNamed', { name: provider.display_name })}><ScanSearch className="mr-1.5 h-3.5 w-3.5" />{t('agentProfiles.models.detect')}</Button><Button type="button" variant="ghost" size="sm" onClick={() => openModelEditor(provider)}><Plus className="mr-1.5 h-3.5 w-3.5" />{t('agentProfiles.models.add')}</Button></div></div>
                                        {provider.models.length > 0 ? <div className="mt-4 space-y-2 border-l border-border/40 pl-3 sm:pl-4">{provider.models.map((model) => {
                                            const isDefault = modelIsDefault(profileForEdit, provider.provider_id, model.model_id);
                                            return (
                                                <div key={model.model_id} className={cn('flex flex-col gap-3 rounded-md bg-muted/25 px-3 py-3 transition-colors hover:bg-muted/50 md:flex-row md:items-center md:justify-between', isDefault && 'bg-primary/[0.06] ring-1 ring-primary/10')}>
                                                    <div className="min-w-0">
                                                        <div className="flex flex-wrap items-center gap-2">
                                                            <Gauge className="h-4 w-4 shrink-0 text-primary" />
                                                            <span className="truncate text-sm font-medium">{model.display_name}</span>
                                                            {!model.enabled && <Badge variant="outline" className="text-[10px]">{t('agentProfiles.models.disabled')}</Badge>}
                                                            {isDefault && <Badge variant="secondary" className="gap-1 text-[10px]"><Check className="h-3 w-3" />{t('agentProfiles.common.default')}</Badge>}
                                                        </div>
                                                        <div className="mt-1 truncate pl-6 font-mono text-[11px] text-muted-foreground">{model.model_id}</div>
                                                        <div className="mt-2 flex flex-wrap items-center gap-1.5 pl-6">
                                                            {model.thinking.options.length > 0 ? model.thinking.options.map((option) => (
                                                                <motion.button key={option} type="button" aria-pressed={model.thinking.selected === option} whileTap={{ scale: 0.93 }} onClick={() => setDraft((current) => current ? updateThinking(current, provider.provider_id, model.model_id, option) : current)} className={cn('rounded-full border px-2.5 py-0.5 text-[11px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring', model.thinking.selected === option ? 'border-primary bg-primary text-primary-foreground shadow-sm' : 'border-border/70 text-muted-foreground hover:border-border hover:bg-accent hover:text-foreground')}>
                                                                    {option}
                                                                </motion.button>
                                                            )) : <span className="text-[11px] text-muted-foreground">{t('agentProfiles.models.noThinkingOptions')}</span>}
                                                        </div>
                                                    </div>
                                                    <div className="flex shrink-0 items-center gap-1 md:pl-4">
                                                        <Button type="button" variant="ghost" size="sm" onClick={() => setDraft((current) => current ? updateManagedModel(current, provider.provider_id, model.model_id) : current)} disabled={isDefault}>{t(isDefault ? 'agentProfiles.models.currentDefault' : 'agentProfiles.actions.setDefault')}</Button>
                                                        <Button type="button" variant="ghost" size="icon" className="h-8 w-8" onClick={() => openModelEditor(provider, model)} title={t('agentProfiles.models.edit')} aria-label={t('agentProfiles.models.editNamed', { name: model.display_name })}><Pencil className="h-3.5 w-3.5" /></Button>
                                                        <Button type="button" variant="ghost" size="icon" className="h-8 w-8 text-destructive hover:text-destructive" onClick={() => openEntityDelete({ kind: 'model', providerId: provider.provider_id, modelId: model.model_id, label: model.display_name })} title={t('agentProfiles.models.delete')} aria-label={t('agentProfiles.models.deleteNamed', { name: model.display_name })}><Trash2 className="h-3.5 w-3.5" /></Button>
                                                    </div>
                                                </div>
                                            );
                                        })}</div> : <div className="mt-4 rounded-md bg-muted/20 px-3 py-4 text-sm text-muted-foreground">{t('agentProfiles.models.empty')}</div>}
                                    </div>
                                ))}
                            </div>
                        </section>

                        {agent === 'claude_code' && (
                            <section className="mt-5 rounded-lg bg-background/45 p-1" aria-label={t('agentProfiles.claudeCompatibility.sectionLabel')}>
                                {(() => {
                                    const advanced = profileForEdit.managed.claude_advanced;
                                    const mainModel = (() => {
                                        const id = profileForEdit.managed.default_model_id;
                                        if (!id) return null;
                                        const slash = id.lastIndexOf('/');
                                        return slash >= 0 ? id.slice(slash + 1) : id;
                                    })();
                                    const capabilityDeclaration = profileForEdit.schema_capability.capability_declaration;
                                    const customModelOptions = profileForEdit.schema_capability.custom_model_options;
                                    const candidateModels = customModelOptions?.status === 'available'
                                        ? Array.from(new Set([
                                            ...customModelOptions.values,
                                            ...profileForEdit.managed.providers.flatMap((provider) => provider.models.filter((model) => model.enabled).map((model) => model.model_id)),
                                        ]))
                                        : [];
                                    const subagentSelection = normalizeClaudeModelSelection(advanced?.subagent_model, candidateModels);
                                    const sonnetSelection = normalizeClaudeModelSelection(advanced?.sonnet_model, candidateModels);
                                    const opusSelection = normalizeClaudeModelSelection(advanced?.opus_model, candidateModels);
                                    const haikuSelection = normalizeClaudeModelSelection(advanced?.haiku_model, candidateModels);
                                    const fableSelection = normalizeClaudeModelSelection(advanced?.fable_model, candidateModels);
                                    const caching = advanced?.disable_prompt_caching;
                                    const renderModelOptions = (selection: ReturnType<typeof normalizeClaudeModelSelection>) => selection.options.map((modelId) => (
                                        <option key={modelId} value={modelId}>
                                            {modelId === CLAUDE_AUTO_MODEL_VALUE
                                                ? t('agentProfiles.claudeCompatibility.autoOption')
                                                : selection.status === 'unknown' && modelId === selection.uiValue
                                                    ? `${modelId} (${t('agentProfiles.claudeCompatibility.unknownOption')})`
                                                    : modelId}
                                        </option>
                                    ));
                                    const updateModelSelection = (field: keyof NonNullable<AgentProfileDocument['managed']['claude_advanced']>, value: string) => {
                                        const selection = normalizeClaudeModelSelection(value, candidateModels);
                                        updateAdvanced({ [field]: selection.payloadValue } as Partial<NonNullable<AgentProfileDocument['managed']['claude_advanced']>>);
                                    };
                                    const updateAdvanced = (patch: Partial<NonNullable<AgentProfileDocument['managed']['claude_advanced']>>) => {
                                        setDraft((current) => {
                                            if (!current) return current;
                                            const next = cloneProfile(current);
                                            next.managed.claude_advanced = { ...(next.managed.claude_advanced || {}), ...patch };
                                            if ('haiku_model' in patch) {
                                                next.managed.claude_advanced.small_fast_model = null;
                                            }
                                            return next;
                                        });
                                    };
                                    return (
                                        <>
                                            <button type="button" onClick={() => setCompatOpen((current) => !current)} className="flex w-full items-center justify-between gap-3 rounded-md px-3 py-3 text-left hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" aria-expanded={compatOpen}><span className="flex items-center gap-2"><Settings2 className="h-4 w-4 text-primary" /><span><span className="block text-sm font-medium">{t('agentProfiles.claudeCompatibility.title')}</span><span className="mt-0.5 block text-xs text-muted-foreground">{t('agentProfiles.claudeCompatibility.description')}</span></span></span><ChevronDown className={cn('h-4 w-4 text-muted-foreground transition-transform', compatOpen && 'rotate-180')} /></button>
                                            <div className="mx-3 mb-2 grid gap-2 rounded-md border border-border/40 bg-muted/20 px-3 py-2 text-[11px] text-muted-foreground md:grid-cols-2">
                                                <div>
                                                    <span className="font-medium text-foreground">{t('agentProfiles.claudeCompatibility.capabilityLabel')}: </span>
                                                    <Badge variant="outline" className={cn('mr-1 px-1.5 py-0 text-[10px]', capabilityDeclaration?.status === 'declared' ? 'border-emerald-500/30 text-emerald-700 dark:text-emerald-300' : 'border-amber-500/30 text-amber-700 dark:text-amber-300')}>
                                                        {capabilityDeclaration?.status === 'declared' ? t('agentProfiles.claudeCompatibility.capabilityDeclared') : t('agentProfiles.claudeCompatibility.capabilityUnavailable')}
                                                    </Badge>
                                                    {capabilityDeclaration?.source || t('agentProfiles.claudeCompatibility.capabilitySourceUnavailable')}
                                                </div>
                                                <div>
                                                    <span className="font-medium text-foreground">{t('agentProfiles.claudeCompatibility.capabilityFieldsLabel')}: </span>
                                                    {capabilityDeclaration?.supported_fields?.join(', ') || t('agentProfiles.claudeCompatibility.capabilitySourceUnavailable')}
                                                </div>
                                                <div>
                                                    <span className="font-medium text-foreground">{t('agentProfiles.claudeCompatibility.customOptionsLabel')}: </span>
                                                    {customModelOptions?.status === 'available' && customModelOptions.values.length > 0
                                                        ? customModelOptions.values.join(', ')
                                                        : customModelOptions?.message || t('agentProfiles.claudeCompatibility.customOptionsUnavailable')}
                                                </div>
                                                <div className="md:col-span-2">
                                                    <span className="font-medium text-foreground">{t('agentProfiles.claudeCompatibility.fallbackPriority')}: </span>
                                                    {(customModelOptions?.fallback_priority || capabilityDeclaration?.fallback_priority || ['explicit_override', 'claude_native_resolution']).join(' → ')}
                                                </div>
                                            </div>
                                            {compatOpen && (
                                                <div className="grid gap-4 pb-4 pt-2 text-xs md:grid-cols-2">
                                                    {mainModel && <div className="md:col-span-2 text-[11px] text-muted-foreground">{t('agentProfiles.claudeCompatibility.currentMainModel', { model: mainModel })}</div>}
                                                    <div>
                                                        <Label htmlFor="claude-subagent-model" className="mb-1.5 block">{t('agentProfiles.claudeCompatibility.subagentModel')}</Label>
                                                        <select id="claude-subagent-model" aria-label={t('agentProfiles.claudeCompatibility.subagentModel')} value={subagentSelection.uiValue} onChange={(event) => updateModelSelection('subagent_model', event.target.value)} className={fieldClass}>
                                                            {renderModelOptions(subagentSelection)}
                                                        </select>
                                                        <p className="mt-1.5 leading-5 text-muted-foreground">{t('agentProfiles.claudeCompatibility.subagentModelHint')}</p>
                                                    </div>
                                                    <div className="md:col-span-2 rounded-md border border-border/40 px-3 py-3">
                                                        <div className="mb-2 text-[11px] font-medium text-muted-foreground">{t('agentProfiles.claudeCompatibility.aliasGroupLabel')}</div>
                                                        <div className="grid gap-4 md:grid-cols-2">
                                                            <div>
                                                                <Label htmlFor="claude-sonnet-model" className="mb-1.5 block">{t('agentProfiles.claudeCompatibility.sonnetModel')}</Label>
                                                                <select id="claude-sonnet-model" aria-label={t('agentProfiles.claudeCompatibility.sonnetModel')} value={sonnetSelection.uiValue} onChange={(event) => updateModelSelection('sonnet_model', event.target.value)} className={fieldClass}>
                                                                    {renderModelOptions(sonnetSelection)}
                                                                </select>
                                                            </div>
                                                            <div>
                                                                <Label htmlFor="claude-opus-model" className="mb-1.5 block">{t('agentProfiles.claudeCompatibility.opusModel')}</Label>
                                                                <select id="claude-opus-model" aria-label={t('agentProfiles.claudeCompatibility.opusModel')} value={opusSelection.uiValue} onChange={(event) => updateModelSelection('opus_model', event.target.value)} className={fieldClass}>
                                                                    {renderModelOptions(opusSelection)}
                                                                </select>
                                                            </div>
                                                            <div>
                                                                <Label htmlFor="claude-haiku-model" className="mb-1.5 block">{t('agentProfiles.claudeCompatibility.haikuModel')}</Label>
                                                                <select id="claude-haiku-model" aria-label={t('agentProfiles.claudeCompatibility.haikuModel')} value={haikuSelection.uiValue} onChange={(event) => updateModelSelection('haiku_model', event.target.value)} className={fieldClass}>
                                                                    {renderModelOptions(haikuSelection)}
                                                                </select>
                                                                <p className="mt-1.5 leading-5 text-muted-foreground">{t('agentProfiles.claudeCompatibility.haikuModelHint')}</p>
                                                            </div>
                                                            <div>
                                                                <Label htmlFor="claude-fable-model" className="mb-1.5 block">{t('agentProfiles.claudeCompatibility.fableModel')}</Label>
                                                                <select id="claude-fable-model" aria-label={t('agentProfiles.claudeCompatibility.fableModel')} value={fableSelection.uiValue} onChange={(event) => updateModelSelection('fable_model', event.target.value)} className={fieldClass}>
                                                                    {renderModelOptions(fableSelection)}
                                                                </select>
                                                            </div>
                                                        </div>
                                                        <p className="mt-2 text-[11px] leading-5 text-muted-foreground">{t('agentProfiles.claudeCompatibility.aliasGroupHint')}</p>
                                                    </div>
                                                    <div className="md:col-span-2">
                                                        <Label htmlFor="claude-disable-caching" className="mb-1.5 block">{t('agentProfiles.claudeCompatibility.disablePromptCaching')}</Label>
                                                        <select id="claude-disable-caching" aria-label={t('agentProfiles.claudeCompatibility.disablePromptCaching')} value={caching === true ? 'on' : caching === false ? 'off' : 'unset'} onChange={(event) => updateAdvanced({ disable_prompt_caching: event.target.value === 'unset' ? null : event.target.value === 'on' })} className={fieldClass}>
                                                            <option value="unset">{t('agentProfiles.common.notSelected')}</option>
                                                            <option value="on">{t('agentProfiles.common.enabled')}</option>
                                                            <option value="off">{t('agentProfiles.common.disabled')}</option>
                                                        </select>
                                                        <p className="mt-1.5 leading-5 text-muted-foreground">{t('agentProfiles.claudeCompatibility.disablePromptCachingHint')}</p>
                                                    </div>
                                                </div>
                                            )}
                                        </>
                                    );
                                })()}
                            </section>
                        )}

                        <section className="mt-5 rounded-lg bg-background/45 p-1" aria-label={t('agentProfiles.advanced.sectionLabel')}>
                            <button type="button" onClick={toggleAdvanced} className="flex w-full items-center justify-between gap-3 rounded-md px-3 py-3 text-left hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" aria-expanded={advancedOpen}><span className="flex items-center gap-2"><FileCode2 className="h-4 w-4 text-primary" /><span><span className="block text-sm font-medium">{t('agentProfiles.advanced.title')}</span><span className="mt-0.5 block text-xs text-muted-foreground">{t('agentProfiles.advanced.description')}</span></span></span><ChevronDown className={cn('h-4 w-4 text-muted-foreground transition-transform', advancedOpen && 'rotate-180')} /></button>
                            {advancedOpen && (
                                <div className="grid gap-4 pb-4 pt-2 text-xs md:grid-cols-2">
                                    <div className="md:col-span-2">
                                        <div className="mb-1.5 flex items-center justify-between gap-2"><span className="text-muted-foreground">{t('agentProfiles.advanced.managedProjection')}</span><Button type="button" size="sm" variant="outline" onClick={applyAdvanced}>{t('agentProfiles.common.applyToDraft')}</Button></div>
                                        <textarea value={advancedText} onChange={(event) => setAdvancedText(event.target.value)} aria-label={t('agentProfiles.advanced.editorLabel')} spellCheck={false} className="min-h-48 w-full resize-y rounded-md border bg-background p-3 font-mono text-[11px] leading-5 outline-none focus-visible:ring-2 focus-visible:ring-ring" />
                                        {advancedError && <div className="mt-2 flex items-start gap-2 text-destructive" role="alert"><AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />{advancedError}</div>}
                                    </div>
                                    <div><div className="mb-1 text-muted-foreground">{t('agentProfiles.advanced.nativeSource')}</div><pre className="overflow-auto whitespace-pre-wrap rounded-md bg-muted/40 p-3 font-mono leading-5">{JSON.stringify({ path: profileForEdit.source.path.native, format: profileForEdit.source.format, scope: profileForEdit.source.scope }, null, 2)}</pre></div>
                                    <div><div className="mb-1 text-muted-foreground">{t('agentProfiles.advanced.protocolDiagnostics')}</div><pre className="overflow-auto whitespace-pre-wrap rounded-md bg-muted/40 p-3 font-mono leading-5">{JSON.stringify({ native: profileForEdit.protocol.native_protocol, upstream: profileForEdit.protocol.upstream_protocol, route: profileForEdit.protocol.route, adapter: profileForEdit.protocol.adapter_id, limitations: profileForEdit.protocol.limitations }, null, 2)}</pre></div>
                                    <div className="md:col-span-2 text-muted-foreground">{t('agentProfiles.advanced.unmanagedNote')}</div>
                                </div>
                            )}
                        </section>

                        {lastSave?.backup_path && <div className="flex flex-col gap-3 py-3 text-sm text-emerald-700 dark:text-emerald-300 md:flex-row md:items-center md:justify-between"><div className="min-w-0"><div className="font-medium">{t('agentProfiles.backup.available')}</div><div className="mt-1 truncate font-mono text-xs">{lastSave.backup_path.display}</div></div><Button type="button" size="sm" variant="outline" onClick={restoreLastSave} disabled={busyAction !== null}><RefreshCw className="mr-2 h-3.5 w-3.5" />{t('agentProfiles.backup.restore')}</Button></div>}
                    </motion.article>
                </div>
            ) : null}

            <Dialog open={profileDialog !== null} onOpenChange={(open) => !open && closeProfileDialog()}>
                <DialogContent className={cn('max-w-md', interactionGroupClass)}>
                    <DialogHeader><DialogTitle>{profileDialogTitle}</DialogTitle><DialogDescription>{profileDialogDescription}</DialogDescription></DialogHeader>
                    {profileDialog === 'delete' ? <>{selectedSummary?.is_default && <div><Label htmlFor="replacement-profile">{t('agentProfiles.dialogs.profile.replacement')}</Label><select id="replacement-profile" value={replacementProfileId} onChange={(event) => setReplacementProfileId(event.target.value)} className={cn(mutedFieldClass, 'mt-1.5')}><option value="">{t('agentProfiles.dialogs.profile.selectReplacement')}</option>{discovery?.profiles.filter((item) => item.profile_id !== selectedProfileId).map((item) => <option key={item.profile_id} value={item.profile_id}>{item.display_name}</option>)}</select></div>}</> : <div><Label htmlFor="profile-name">{t('agentProfiles.dialogs.profile.name')}</Label><Input id="profile-name" value={profileDialogName} onChange={(event) => setProfileDialogName(event.target.value)} className="mt-1.5" autoFocus /></div>}
                    {profileDialogError && <div className="flex items-start gap-2 text-sm text-destructive" role="alert"><AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />{profileDialogError}</div>}
                    <DialogFooter><Button type="button" variant="outline" onClick={closeProfileDialog} disabled={busyAction === 'crud'}>{t('agentProfiles.common.cancel')}</Button><Button type="button" variant={profileDialog === 'delete' ? 'destructive' : 'default'} onClick={submitProfileDialog} disabled={busyAction === 'crud'}>{busyAction === 'crud' && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}{t(profileDialog === 'delete' ? 'agentProfiles.common.confirmDelete' : 'agentProfiles.common.save')}</Button></DialogFooter>
                </DialogContent>
            </Dialog>

            <Dialog open={entityDelete !== null} onOpenChange={(open) => !open && setEntityDelete(null)}>
                <DialogContent className={cn('max-w-md', interactionGroupClass)}><DialogHeader><DialogTitle>{t(entityDelete?.kind === 'provider' ? 'agentProfiles.dialogs.entityDelete.providerTitle' : 'agentProfiles.dialogs.entityDelete.modelTitle')}</DialogTitle><DialogDescription>{t('agentProfiles.dialogs.entityDelete.description', { name: entityDelete?.label || '' })}</DialogDescription></DialogHeader><DialogFooter><Button type="button" variant="outline" onClick={() => setEntityDelete(null)}>{t('agentProfiles.common.cancel')}</Button><Button type="button" variant="destructive" onClick={submitEntityDelete}>{t('agentProfiles.common.confirmDelete')}</Button></DialogFooter></DialogContent>
            </Dialog>

            <Dialog open={providerEditor !== null} onOpenChange={(open) => !open && closeProviderEditor()}>
                <DialogContent className={cn('max-w-2xl', interactionGroupClass)}>
                    <DialogHeader><DialogTitle>{t(providerEditor?.mode === 'create' ? 'agentProfiles.dialogs.provider.createTitle' : 'agentProfiles.dialogs.provider.editTitle')}</DialogTitle><DialogDescription>{t('agentProfiles.dialogs.provider.descriptionClaude')}</DialogDescription></DialogHeader>
                    {providerForm && (
                        <div className="grid gap-4 md:grid-cols-2">
                            <div><Label htmlFor="provider-id">Provider ID</Label><Input id="provider-id" value={providerForm.provider_id} onChange={(event) => setProviderForm({ ...providerForm, provider_id: event.target.value })} className="mt-1.5" readOnly={providerEditor?.mode === 'edit'} /></div>
                            <div><Label htmlFor="provider-name">{t('agentProfiles.forms.displayName')}</Label><Input id="provider-name" value={providerForm.display_name} onChange={(event) => setProviderForm({ ...providerForm, display_name: event.target.value })} className="mt-1.5" /></div>
                            <div className="md:col-span-2"><Label htmlFor="provider-base-url">Base URL</Label><Input id="provider-base-url" value={providerForm.base_url} onChange={(event) => setProviderForm({ ...providerForm, base_url: event.target.value })} placeholder="https://api.example.com/v1" className="mt-1.5 font-mono" /></div>
                            <div className="md:col-span-2"><Label htmlFor="provider-api-key">{t('agentProfiles.forms.apiKey')}</Label><Input id="provider-api-key" type="password" autoComplete="off" value={providerForm.api_key} onChange={(event) => setProviderForm({ ...providerForm, api_key: event.target.value, clear_api_key: false })} placeholder={t('agentProfiles.forms.apiKeyPlaceholder')} className="mt-1.5 font-mono" /></div>
                            <label className="flex items-center gap-2 text-sm md:col-span-2"><Switch checked={providerForm.clear_api_key} onCheckedChange={(checked) => setProviderForm({ ...providerForm, clear_api_key: checked, api_key: checked ? '' : providerForm.api_key })} />{t('agentProfiles.forms.clearApiKey')}</label>
                            {agent !== 'claude_code' && (
                                <>
                                    <div><Label htmlFor="provider-native-protocol">{t('agentProfiles.forms.nativeProtocol')}</Label><select id="provider-native-protocol" value={providerForm.protocol.native_protocol} onChange={(event) => setProviderForm({ ...providerForm, protocol: { ...providerForm.protocol, native_protocol: event.target.value as ProtocolCapability['native_protocol'] } })} className={cn(fieldClass, 'mt-1.5')}><option value="openai_responses">OpenAI Responses</option><option value="openai_chat_completions">OpenAI Chat Completions</option><option value="anthropic_messages">Anthropic Messages</option><option value="unknown">{t('agentProfiles.common.unknown')}</option></select></div>
                                    <div><Label htmlFor="provider-upstream-protocol">{t('agentProfiles.forms.upstreamProtocol')}</Label><select id="provider-upstream-protocol" value={providerForm.protocol.upstream_protocol} onChange={(event) => setProviderForm({ ...providerForm, protocol: { ...providerForm.protocol, upstream_protocol: event.target.value as ProtocolCapability['upstream_protocol'] } })} className={cn(fieldClass, 'mt-1.5')}><option value="openai_responses">OpenAI Responses</option><option value="openai_chat_completions">OpenAI Chat Completions</option><option value="anthropic_messages">Anthropic Messages</option><option value="unknown">{t('agentProfiles.common.unknown')}</option></select></div>
                                    <p className="text-xs leading-5 text-muted-foreground md:col-span-2">{t('agentProfiles.dialogs.provider.protocolNote')}</p>
                                </>
                            )}
                        </div>
                    )}
                    {providerError && <div className="flex items-start gap-2 text-sm text-destructive" role="alert"><AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />{providerError}</div>}
                    <DialogFooter><Button type="button" variant="outline" onClick={closeProviderEditor}>{t('agentProfiles.common.cancel')}</Button><Button type="button" onClick={submitProviderEditor}>{t('agentProfiles.common.applyToDraft')}</Button></DialogFooter>
                </DialogContent>
            </Dialog>

            <Dialog open={modelEditor !== null} onOpenChange={(open) => !open && closeModelEditor()}>
                <DialogContent className={cn('max-w-2xl', interactionGroupClass)}>
                    <DialogHeader><DialogTitle>{t(modelEditor?.mode === 'create' ? 'agentProfiles.dialogs.model.createTitle' : 'agentProfiles.dialogs.model.editTitle')}</DialogTitle><DialogDescription>{t('agentProfiles.dialogs.model.description')}</DialogDescription></DialogHeader>
                    {modelForm && (
                        <div className="space-y-4">
                            <div className="grid gap-4 md:grid-cols-2">
                                <div><Label htmlFor="model-id">Model ID</Label><Input id="model-id" value={modelForm.model_id} onChange={(event) => setModelForm({ ...modelForm, model_id: event.target.value })} className="mt-1.5 font-mono" readOnly={modelEditor?.mode === 'edit' && agent !== 'opencode'} /></div>
                                <div><Label htmlFor="model-name">{t('agentProfiles.forms.displayName')}</Label><Input id="model-name" value={modelForm.display_name} onChange={(event) => setModelForm({ ...modelForm, display_name: event.target.value })} className="mt-1.5" /></div>
                            </div>
                            <div className="flex flex-wrap gap-5 py-2">
                                <label className="flex items-center gap-2 text-sm"><Switch checked={modelForm.enabled} onCheckedChange={(checked) => setModelForm({ ...modelForm, enabled: checked })} />{t('agentProfiles.forms.enableModel')}</label>
                                <label className="flex items-center gap-2 text-sm"><Switch checked={modelForm.supports_reasoning} onCheckedChange={(checked) => setModelForm({ ...modelForm, supports_reasoning: checked })} />{t('agentProfiles.forms.supportsReasoning')}</label>
                                <label className="flex items-center gap-2 text-sm"><Switch checked={modelForm.supports_effort} onCheckedChange={(checked) => setModelForm({ ...modelForm, supports_effort: checked })} />{t('agentProfiles.forms.supportsEffort')}</label>
                            </div>
                            <div>
                                <div className="flex items-center justify-between gap-2">
                                    <div><Label>{t('agentProfiles.forms.thinkingOptions')}</Label><p className="mt-1 text-xs text-muted-foreground">{t('agentProfiles.forms.thinkingOptionsHint')}</p></div>
                                    {modelForm.options.length > 0 && <select aria-label={t('agentProfiles.forms.defaultThinking')} value={modelForm.selected} onChange={(event) => setModelForm({ ...modelForm, selected: event.target.value })} className="h-9 rounded-md border border-input bg-background px-2 text-xs"><option value="">{t('agentProfiles.common.notSelected')}</option>{modelForm.options.map((option) => <option key={option} value={option}>{option}</option>)}</select>}
                                </div>
                                <div className="mt-3 flex flex-wrap gap-2">
                                    {modelForm.options.map((option) => <span key={option} className={cn('inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-xs', modelForm.selected === option && 'border-primary bg-primary/10 text-primary')}><button type="button" onClick={() => setModelForm({ ...modelForm, selected: option })} className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{option}</button><button type="button" aria-label={t('agentProfiles.forms.deleteVariant', { option })} onClick={() => setModelForm({ ...modelForm, options: modelForm.options.filter((item) => item !== option), selected: modelForm.selected === option ? '' : modelForm.selected })} className="text-muted-foreground hover:text-destructive">×</button></span>)}
                                    <div className="flex items-center gap-1"><Input value={variantInput} onChange={(event) => setVariantInput(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); const value = variantInput.trim(); if (value && !modelForm.options.includes(value)) setModelForm({ ...modelForm, options: [...modelForm.options, value], selected: modelForm.selected || value }); setVariantInput(''); } }} placeholder={t('agentProfiles.forms.addVariant')} className="h-8 w-32 text-xs" /><Button type="button" variant="outline" size="sm" onClick={() => { const value = variantInput.trim(); if (value && !modelForm.options.includes(value)) setModelForm({ ...modelForm, options: [...modelForm.options, value], selected: modelForm.selected || value }); setVariantInput(''); }}>{t('agentProfiles.common.add')}</Button></div>
                                </div>
                            </div>
                            <label className="flex items-center gap-2 text-sm"><Switch checked={modelForm.custom_allowed} onCheckedChange={(checked) => setModelForm({ ...modelForm, custom_allowed: checked })} />{t('agentProfiles.forms.allowCustomEffort')}</label>
                            <p className="text-xs leading-5 text-muted-foreground">{t('agentProfiles.dialogs.model.capabilityNote')}</p>
                        </div>
                    )}
                    {modelError && <div className="flex items-start gap-2 text-sm text-destructive" role="alert"><AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />{modelError}</div>}
                    <DialogFooter><Button type="button" variant="outline" onClick={closeModelEditor}>{t('agentProfiles.common.cancel')}</Button><Button type="button" onClick={submitModelEditor}>{t('agentProfiles.common.applyToDraft')}</Button></DialogFooter>
                </DialogContent>
            </Dialog>

            <Dialog open={modelImport !== null} onOpenChange={(open) => !open && closeModelImport()}>
                <DialogContent className={cn('max-w-xl', interactionGroupClass)}>
                    <DialogHeader>
                        <DialogTitle>{t('agentProfiles.dialogs.modelImport.title')}</DialogTitle>
                        <DialogDescription>{t('agentProfiles.dialogs.modelImport.description')}</DialogDescription>
                    </DialogHeader>
                    {modelImport && (
                        <div className="space-y-3">
                            {modelImport.loading ? (
                                <div className="flex items-center gap-2 py-6 text-sm text-muted-foreground"><Loader2 className="h-4 w-4 animate-spin" />{t('agentProfiles.models.detecting')}</div>
                            ) : (
                                <>
                                    {modelImport.endpoint && <p className="truncate font-mono text-[11px] text-muted-foreground">{modelImport.endpoint}</p>}
                                    {modelImport.items.length > 0 && (
                                        <>
                                            <Input value={modelImport.filter} onChange={(event) => setModelImport({ ...modelImport, filter: event.target.value })} placeholder={t('agentProfiles.models.filter')} aria-label={t('agentProfiles.models.filter')} />
                                            <div className="flex flex-wrap gap-2">
                                                <Button type="button" variant="outline" size="sm" onClick={() => {
                                                    const visible = modelImport.items.filter((item) => {
                                                        const query = modelImport.filter.trim().toLowerCase();
                                                        if (!query) return !item.imported;
                                                        return !item.imported && `${item.model_id} ${item.display_name}`.toLowerCase().includes(query);
                                                    }).map((item) => item.model_id);
                                                    setModelImport({ ...modelImport, selected: Array.from(new Set([...modelImport.selected, ...visible])) });
                                                }}>{t('agentProfiles.models.selectUnimported')}</Button>
                                                <Button type="button" variant="ghost" size="sm" onClick={() => setModelImport({ ...modelImport, selected: [] })}>{t('agentProfiles.models.clearSelection')}</Button>
                                            </div>
                                            <div className="max-h-72 space-y-1 overflow-y-auto pr-1" role="list">
                                                {modelImport.items.filter((item) => {
                                                    const query = modelImport.filter.trim().toLowerCase();
                                                    return !query || `${item.model_id} ${item.display_name}`.toLowerCase().includes(query);
                                                }).map((item, index) => {
                                                    const checked = modelImport.selected.includes(item.model_id);
                                                    return (
                                                        <label key={item.model_id} role="listitem" htmlFor={`upstream-model-${index}`} className={cn('flex cursor-pointer items-start gap-2 rounded-md px-2 py-2 text-sm hover:bg-muted/50', item.imported && 'cursor-default opacity-60')}>
                                                            <Checkbox id={`upstream-model-${index}`} checked={checked || item.imported} disabled={item.imported} onCheckedChange={(value) => {
                                                                if (item.imported) return;
                                                                const selected = new Set(modelImport.selected);
                                                                if (value === true) selected.add(item.model_id);
                                                                else selected.delete(item.model_id);
                                                                setModelImport({ ...modelImport, selected: Array.from(selected) });
                                                            }} />
                                                            <span className="min-w-0 flex-1">
                                                                <span className="block truncate">{item.display_name}</span>
                                                                <span className="block truncate font-mono text-[11px] text-muted-foreground">{item.model_id}</span>
                                                            </span>
                                                            {item.imported && <Badge variant="outline" className="shrink-0 text-[10px]">{t('agentProfiles.models.alreadyImported')}</Badge>}
                                                        </label>
                                                    );
                                                })}
                                            </div>
                                        </>
                                    )}
                                    {!modelImport.error && modelImport.items.length === 0 && <p className="py-4 text-sm text-muted-foreground">{t('agentProfiles.models.noneFound')}</p>}
                                </>
                            )}
                            {modelImport.error && <div className="flex items-start gap-2 text-sm text-destructive" role="alert"><AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />{modelImport.error}</div>}
                        </div>
                    )}
                    <DialogFooter>
                        <Button type="button" variant="outline" onClick={closeModelImport}>{t('agentProfiles.common.cancel')}</Button>
                        <Button type="button" onClick={submitModelImport} disabled={!modelImport || modelImport.loading || modelImport.selected.filter((id) => modelImport.items.some((item) => item.model_id === id && !item.imported)).length === 0}>{t('agentProfiles.models.importCount', { count: modelImport?.selected.filter((id) => modelImport.items.some((item) => item.model_id === id && !item.imported)).length || 0 })}</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </section>
    );
}
