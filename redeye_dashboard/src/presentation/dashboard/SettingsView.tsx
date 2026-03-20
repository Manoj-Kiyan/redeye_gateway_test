// Dashboard View - SettingsView

import { Copy, KeyRound, Plus, Save, Settings as SettingsIcon, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { authService, type AuditLogEntry, type ProviderCatalogEntry, type ProviderStatus, type RouteDryRunResult, type TenantMember, type TenantRoute } from '../../data/services/authService';
import { ErrorBanner } from '../components/ui/ErrorBanner';
import { useAuth } from '../context/AuthContext';

const EMPTY_ROUTE: TenantRoute = {
  provider: 'openai',
  model: '',
  isDefault: true,
};

export function SettingsView() {
  const { user } = useAuth();
  const [gatewayUrl, setGatewayUrl] = useState('http://localhost:8080');
  const [cacheUrl, setCacheUrl] = useState('http://localhost:8081');
  const [tracerUrl, setTracerUrl] = useState('http://localhost:8082');
  const [complianceUrl, setComplianceUrl] = useState('http://localhost:8083');
  const [providerStatus, setProviderStatus] = useState<ProviderStatus | null>(null);
  const [providerCatalog, setProviderCatalog] = useState<ProviderCatalogEntry[]>([]);
  const [routes, setRoutes] = useState<TenantRoute[]>([EMPTY_ROUTE]);
  const [auditLogs, setAuditLogs] = useState<AuditLogEntry[]>([]);
  const [members, setMembers] = useState<TenantMember[]>([]);
  const [routeDryRunModel, setRouteDryRunModel] = useState('');
  const [routeDryRunResult, setRouteDryRunResult] = useState<RouteDryRunResult | null>(null);
  const [providerForm, setProviderForm] = useState({
    openAiApiKey: '',
    anthropicApiKey: '',
    geminiApiKey: '',
  });
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [savingProviders, setSavingProviders] = useState(false);
  const [savingRoutes, setSavingRoutes] = useState(false);
  const [runningDryRun, setRunningDryRun] = useState(false);
  const [updatingMemberId, setUpdatingMemberId] = useState<string | null>(null);
  const canManageProviders = user?.role === 'owner' || user?.role === 'admin';
  const canManageRoutes = user?.role === 'owner' || user?.role === 'admin';
  const canViewMembers = user?.role === 'owner' || user?.role === 'admin';
  const canManageMembers = user?.role === 'owner';

  useEffect(() => {
    async function loadSettings() {
      setLoading(true);
      setError(null);

      try {
        const [status, tenantRoutes, catalog, recentAuditLogs, tenantMembers] = await Promise.all([
          authService.getProviderStatus(),
          authService.getTenantRoutes(),
          authService.getProviderCatalog(),
          authService.getAuditLogs(),
          canViewMembers ? authService.getTenantMembers() : Promise.resolve([]),
        ]);

        setProviderStatus(status);
        setRoutes(tenantRoutes.length > 0 ? tenantRoutes : [EMPTY_ROUTE]);
        setProviderCatalog(catalog);
        setAuditLogs(recentAuditLogs);
        setMembers(tenantMembers);
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : 'Failed to load settings.');
      } finally {
        setLoading(false);
      }
    }

    void loadSettings();
  }, [canViewMembers]);

  const providerCards = useMemo(() => {
    const fallback = user?.providerStatus;
    return [
      { label: 'OpenAI', configured: providerStatus?.openaiConfigured ?? fallback?.openaiConfigured ?? false, desc: 'Default production-ready provider path.' },
      { label: 'Anthropic', configured: providerStatus?.anthropicConfigured ?? fallback?.anthropicConfigured ?? false, desc: 'Claude model routing support.' },
      { label: 'Gemini', configured: providerStatus?.geminiConfigured ?? fallback?.geminiConfigured ?? false, desc: 'Google Gemini model routing support.' },
    ];
  }, [providerStatus, user]);

  const catalogByProvider = useMemo(() => {
    return providerCatalog.reduce<Record<string, string[]>>((acc, entry) => {
      acc[entry.provider] = entry.suggestedModels;
      return acc;
    }, {});
  }, [providerCatalog]);

  async function refreshAuditLogs() {
    setAuditLogs(await authService.getAuditLogs());
  }

  async function refreshMembers() {
    if (!canViewMembers) return;
    setMembers(await authService.getTenantMembers());
  }

  async function copyGatewayKey() {
    const key = providerStatus?.redeyeApiKey ?? user?.redeyeApiKey;
    if (!key) return;
    await navigator.clipboard.writeText(key);
    setSuccess('Gateway key copied to clipboard.');
  }

  function updateRoute(index: number, next: Partial<TenantRoute>) {
    setRoutes((prev) => prev.map((route, routeIndex) => {
      if (routeIndex !== index) return route;
      return { ...route, ...next };
    }));
  }

  function setDefaultRoute(index: number) {
    setRoutes((prev) => prev.map((route, routeIndex) => ({
      ...route,
      isDefault: routeIndex === index,
    })));
  }

  function addRoute() {
    setRoutes((prev) => [...prev, { ...EMPTY_ROUTE, isDefault: prev.length === 0 }]);
  }

  function removeRoute(index: number) {
    setRoutes((prev) => {
      const next = prev.filter((_, routeIndex) => routeIndex !== index);
      if (next.length === 0) return [{ ...EMPTY_ROUTE, isDefault: true }];
      if (!next.some((route) => route.isDefault)) {
        next[0] = { ...next[0], isDefault: true };
      }
      return next;
    });
  }

  async function handleProviderSave() {
    setSavingProviders(true);
    setError(null);
    setSuccess(null);

    try {
      const status = await authService.updateProviderCredentials({
        openAiApiKey: providerForm.openAiApiKey.trim() || undefined,
        anthropicApiKey: providerForm.anthropicApiKey.trim() || undefined,
        geminiApiKey: providerForm.geminiApiKey.trim() || undefined,
      });
      setProviderStatus(status);
      await refreshAuditLogs();
      setProviderForm({ openAiApiKey: '', anthropicApiKey: '', geminiApiKey: '' });
      setSuccess('Provider credentials updated successfully.');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to update provider credentials.');
    } finally {
      setSavingProviders(false);
    }
  }

  async function handleRouteSave() {
    setSavingRoutes(true);
    setError(null);
    setSuccess(null);

    try {
      const cleanedRoutes = routes.map((route) => ({
        provider: route.provider,
        model: route.model.trim(),
        isDefault: route.isDefault,
      }));
      const updatedRoutes = await authService.updateTenantRoutes(cleanedRoutes);
      setRoutes(updatedRoutes.length > 0 ? updatedRoutes : [EMPTY_ROUTE]);
      await refreshAuditLogs();
      setSuccess('Tenant routes updated successfully.');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to update tenant routes.');
    } finally {
      setSavingRoutes(false);
    }
  }

  async function handleRouteDryRun() {
    setRunningDryRun(true);
    setError(null);
    setSuccess(null);

    try {
      const result = await authService.dryRunTenantRoute(routeDryRunModel.trim());
      setRouteDryRunResult(result);
      setSuccess('Route dry-run completed successfully.');
    } catch (err: unknown) {
      setRouteDryRunResult(null);
      setError(err instanceof Error ? err.message : 'Failed to preview tenant route.');
    } finally {
      setRunningDryRun(false);
    }
  }

  async function handleMemberRoleChange(memberId: string, role: TenantMember['role']) {
    setUpdatingMemberId(memberId);
    setError(null);
    setSuccess(null);

    try {
      await authService.updateMemberRole(memberId, role);
      await Promise.all([refreshMembers(), refreshAuditLogs()]);
      setSuccess('Member role updated successfully.');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to update member role.');
    } finally {
      setUpdatingMemberId(null);
    }
  }

  return (
    <div className="space-y-6">
      <header>
        <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Configuration</p>
        <h1 className="text-2xl sm:text-3xl font-bold text-slate-50">Gateway Settings</h1>
        <p className="text-sm text-slate-400 mt-1">Manage provider credentials, tenant model routes, and service targets.</p>
        <div className="mt-3 inline-flex items-center gap-2 rounded-full border border-slate-800 bg-slate-900/60 px-3 py-1.5 text-xs text-slate-300">
          <span className="text-slate-500">Access role</span>
          <span className="font-semibold uppercase tracking-[0.18em] text-indigo-300">{user?.role ?? 'viewer'}</span>
        </div>
      </header>

      {error && <ErrorBanner error={error} type="error" onClose={() => setError(null)} />}
      {success && <ErrorBanner error={success} type="warning" onClose={() => setSuccess(null)} />}

      <div className="grid grid-cols-1 xl:grid-cols-3 gap-4 sm:gap-6">
        <div className="xl:col-span-2 grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
          {[
            { label: 'Gateway', desc: 'Traffic, rate limiting and provider orchestration.', value: gatewayUrl, set: setGatewayUrl, def: '8080' },
            { label: 'Semantic Cache', desc: 'Vector-aware cache for repeated prompts.', value: cacheUrl, set: setCacheUrl, def: '8081' },
            { label: 'Tracer', desc: 'Distributed traces and audit-grade spans.', value: tracerUrl, set: setTracerUrl, def: '8082' },
            { label: 'Compliance', desc: 'PII redaction and residency enforcement.', value: complianceUrl, set: setComplianceUrl, def: '8083' },
          ].map(({ label, desc, value, set, def }) => (
            <div key={label} className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
              <p className="text-xs font-medium text-slate-400 mb-1">{label}</p>
              <p className="text-sm text-slate-300 mb-3">{desc}</p>
              <input
                className="w-full rounded-md bg-slate-950/60 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                value={value}
                onChange={(e) => set(e.target.value)}
              />
              <p className="text-[11px] text-slate-500 mt-2">Default: http://localhost:{def}</p>
            </div>
          ))}
        </div>

        <div className="space-y-4">
          <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
            <div className="flex items-center gap-2 mb-3">
              <KeyRound className="w-4 h-4 text-indigo-400" />
              <h2 className="text-sm font-semibold text-slate-100">Tenant Gateway Key</h2>
            </div>
            <p className="text-xs text-slate-400 mb-3">Use this `re-sk-...` key in client apps instead of raw provider keys.</p>
            <div className="rounded-md border border-slate-800 bg-slate-950/60 px-3 py-2 font-mono text-xs text-slate-200 break-all">
              {providerStatus?.redeyeApiKey ?? user?.redeyeApiKey ?? 'Key will appear after onboarding or provider refresh.'}
            </div>
            <button
              type="button"
              onClick={copyGatewayKey}
              disabled={!providerStatus?.redeyeApiKey && !user?.redeyeApiKey}
              className="mt-3 inline-flex items-center gap-2 rounded-md border border-slate-700 px-3 py-2 text-xs font-semibold text-slate-200 hover:bg-slate-800/60 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <Copy className="w-3.5 h-3.5" />
              Copy key
            </button>
          </div>

          <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5">
            <h2 className="text-sm font-semibold text-slate-100 mb-3">Provider Status</h2>
            <div className="space-y-3">
              {providerCards.map((provider) => (
                <div key={provider.label} className="rounded-md border border-slate-800 bg-slate-950/40 px-3 py-3">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium text-slate-100">{provider.label}</p>
                      <p className="text-xs text-slate-500 mt-1">{provider.desc}</p>
                    </div>
                    <span className={`rounded-full px-2 py-1 text-[10px] font-semibold ${provider.configured ? 'bg-emerald-500/10 text-emerald-300 ring-1 ring-emerald-500/20' : 'bg-amber-500/10 text-amber-300 ring-1 ring-amber-500/20'}`}>
                      {provider.configured ? 'Configured' : 'Pending'}
                    </span>
                  </div>
                </div>
              ))}
            </div>
            <p className="text-[11px] text-slate-500 mt-3">{loading ? 'Loading provider status...' : 'Provider status now comes from live auth APIs.'}</p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 space-y-4">
          <div>
            <h2 className="text-lg font-semibold text-slate-100">Provider Credentials</h2>
            <p className="text-sm text-slate-400 mt-1">Update tenant-scoped provider keys. Empty fields are ignored.</p>
            {!canManageProviders && (
              <p className="text-xs text-amber-300 mt-2">Viewer access can inspect provider status, but cannot rotate credentials.</p>
            )}
          </div>

          <CredentialInput disabled={!canManageProviders} label="OpenAI API Key" value={providerForm.openAiApiKey} onChange={(value) => setProviderForm((prev) => ({ ...prev, openAiApiKey: value }))} placeholder="sk-..." />
          <CredentialInput disabled={!canManageProviders} label="Anthropic API Key" value={providerForm.anthropicApiKey} onChange={(value) => setProviderForm((prev) => ({ ...prev, anthropicApiKey: value }))} placeholder="sk-ant-..." />
          <CredentialInput disabled={!canManageProviders} label="Gemini API Key" value={providerForm.geminiApiKey} onChange={(value) => setProviderForm((prev) => ({ ...prev, geminiApiKey: value }))} placeholder="AIza..." />

          <div className="flex justify-end">
            <button
              type="button"
              onClick={handleProviderSave}
              disabled={savingProviders || !canManageProviders}
              className="inline-flex items-center gap-2 rounded-md bg-indigo-600 px-4 py-2 text-xs font-semibold text-white hover:bg-indigo-500 disabled:opacity-60 transition-colors"
            >
              <Save className="w-3.5 h-3.5" />
              {savingProviders ? 'Saving...' : 'Save provider keys'}
            </button>
          </div>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 space-y-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold text-slate-100">Tenant Model Routes</h2>
              <p className="text-sm text-slate-400 mt-1">Define which provider/model combinations this tenant can use.</p>
            </div>
            <button
              type="button"
              onClick={addRoute}
              disabled={!canManageRoutes}
              className="inline-flex items-center gap-2 rounded-md border border-slate-700 px-3 py-2 text-xs font-semibold text-slate-200 hover:bg-slate-800/60 transition-colors"
            >
              <Plus className="w-3.5 h-3.5" />
              Add route
            </button>
          </div>

          <div className="space-y-3">
            {routes.map((route, index) => (
              <div key={`${route.provider}-${index}`} className="rounded-md border border-slate-800 bg-slate-950/40 p-4 space-y-3">
                <div className="grid grid-cols-1 md:grid-cols-[140px_1fr_auto_auto] gap-3 items-center">
                  <select
                    value={route.provider}
                    onChange={(e) => updateRoute(index, { provider: e.target.value as TenantRoute['provider'] })}
                    disabled={!canManageRoutes}
                    className="rounded-md bg-slate-950/70 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                  >
                    <option value="openai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                    <option value="gemini">Gemini</option>
                  </select>
                  <input
                    value={route.model}
                    onChange={(e) => updateRoute(index, { model: e.target.value })}
                    disabled={!canManageRoutes}
                    placeholder="e.g. gpt-4o-mini"
                    list={`provider-models-${index}`}
                    className="rounded-md bg-slate-950/70 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                  />
                  <datalist id={`provider-models-${index}`}>
                    {(catalogByProvider[route.provider] ?? []).map((model) => (
                      <option key={model} value={model} />
                    ))}
                  </datalist>
                  <label className="inline-flex items-center gap-2 text-xs text-slate-300">
                    <input
                      type="radio"
                      name="default-route"
                      checked={route.isDefault}
                      disabled={!canManageRoutes}
                      onChange={() => setDefaultRoute(index)}
                    />
                    Default
                  </label>
                  <button
                    type="button"
                    onClick={() => removeRoute(index)}
                    disabled={!canManageRoutes}
                    className="inline-flex items-center justify-center rounded-md border border-rose-500/20 px-3 py-2 text-rose-300 hover:bg-rose-500/10 transition-colors"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            ))}
          </div>

          <div className="flex justify-end">
            <button
              type="button"
              onClick={handleRouteSave}
              disabled={savingRoutes || !canManageRoutes}
              className="inline-flex items-center gap-2 rounded-md bg-indigo-600 px-4 py-2 text-xs font-semibold text-white hover:bg-indigo-500 disabled:opacity-60 transition-colors"
            >
              <Save className="w-3.5 h-3.5" />
              {savingRoutes ? 'Saving...' : 'Save routes'}
            </button>
          </div>

          <div className="border-t border-slate-800 pt-4 space-y-3">
            <div>
              <h3 className="text-sm font-semibold text-slate-100">Route Sandbox</h3>
              <p className="text-xs text-slate-500 mt-1">Preview how a model resolves for this tenant before wiring client traffic.</p>
            </div>

            <div className="flex flex-col sm:flex-row gap-3">
              <input
                value={routeDryRunModel}
                onChange={(e) => setRouteDryRunModel(e.target.value)}
                placeholder="e.g. gpt-4o-mini"
                className="flex-1 rounded-md bg-slate-950/70 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              />
              <button
                type="button"
                onClick={handleRouteDryRun}
                disabled={runningDryRun || !routeDryRunModel.trim()}
                className="inline-flex items-center justify-center rounded-md border border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:bg-slate-800/60 disabled:opacity-60 transition-colors"
              >
                {runningDryRun ? 'Testing...' : 'Test route'}
              </button>
            </div>

            {routeDryRunResult && (
              <div className="rounded-md border border-slate-800 bg-slate-950/50 p-4 text-sm text-slate-200 space-y-1">
                <p><span className="text-slate-500">Requested:</span> {routeDryRunResult.requestedModel}</p>
                <p><span className="text-slate-500">Resolved provider:</span> {routeDryRunResult.resolvedProvider}</p>
                <p><span className="text-slate-500">Effective model:</span> {routeDryRunResult.effectiveModel}</p>
                <p><span className="text-slate-500">Tenant route configured:</span> {routeDryRunResult.routeConfigured ? 'Yes' : 'No'}</p>
                <p><span className="text-slate-500">Provider credential ready:</span> {routeDryRunResult.providerCredentialConfigured ? 'Yes' : 'No'}</p>
              </div>
            )}
          </div>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 space-y-4">
          <div>
            <h2 className="text-lg font-semibold text-slate-100">Team Access</h2>
            <p className="text-sm text-slate-400 mt-1">Owner can change roles. Admin can review members. Viewer access stays read-only.</p>
          </div>

          {!canViewMembers ? (
            <div className="rounded-md border border-dashed border-slate-800 bg-slate-950/30 px-4 py-6 text-sm text-slate-500">
              Your current role can use the platform, but team membership is visible only to admin and owner roles.
            </div>
          ) : members.length === 0 ? (
            <div className="rounded-md border border-dashed border-slate-800 bg-slate-950/30 px-4 py-6 text-sm text-slate-500">
              No team members found for this tenant yet.
            </div>
          ) : (
            <div className="space-y-3">
              {members.map((member) => {
                const isSelf = member.id === user?.id;

                return (
                  <div key={member.id} className="rounded-md border border-slate-800 bg-slate-950/40 p-4">
                    <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                      <div>
                        <p className="text-sm font-medium text-slate-100">{member.email}</p>
                        <p className="text-xs text-slate-500 mt-1">Joined {new Date(member.createdAt).toLocaleString()}</p>
                      </div>
                      <div className="flex items-center gap-3">
                        {canManageMembers ? (
                          <select
                            value={member.role}
                            disabled={updatingMemberId === member.id || isSelf}
                            onChange={(e) => void handleMemberRoleChange(member.id, e.target.value as TenantMember['role'])}
                            className="rounded-md bg-slate-950/70 border border-slate-800 px-3 py-2 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:opacity-60"
                          >
                            <option value="owner">Owner</option>
                            <option value="admin">Admin</option>
                            <option value="viewer">Viewer</option>
                          </select>
                        ) : (
                          <span className="rounded-full bg-slate-800 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.16em] text-slate-300">
                            {member.role}
                          </span>
                        )}
                        {isSelf && (
                          <span className="rounded-full bg-indigo-500/10 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-indigo-300 ring-1 ring-indigo-500/20">
                            You
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 space-y-4">
          <div>
            <h2 className="text-lg font-semibold text-slate-100">Audit Trail</h2>
            <p className="text-sm text-slate-400 mt-1">Recent provider and route changes for this tenant.</p>
          </div>

          <div className="space-y-3 max-h-[420px] overflow-auto pr-1">
            {auditLogs.length === 0 ? (
              <div className="rounded-md border border-dashed border-slate-800 bg-slate-950/30 px-4 py-6 text-sm text-slate-500">
                No admin changes recorded yet.
              </div>
            ) : (
              auditLogs.map((entry) => (
                <div key={entry.id} className="rounded-md border border-slate-800 bg-slate-950/40 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium text-slate-100">{entry.action.replaceAll('_', ' ')}</p>
                      <p className="text-xs text-slate-500 mt-1">{entry.service} · {entry.targetType}</p>
                    </div>
                    <p className="text-[11px] text-slate-500">{new Date(entry.createdAt).toLocaleString()}</p>
                  </div>
                  <pre className="mt-3 overflow-auto rounded-md bg-slate-950/70 p-3 text-[11px] text-slate-300">
                    {JSON.stringify(entry.metadata, null, 2)}
                  </pre>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      <div className="flex items-center justify-end">
        <button
          type="button"
          className="inline-flex items-center gap-2 rounded-md bg-slate-100/5 border border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:bg-slate-100/10 transition-colors cursor-default"
        >
          <SettingsIcon className="w-3 h-3" />
          <span>Endpoint settings are local to this session</span>
        </button>
      </div>
    </div>
  );
}

interface CredentialInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  disabled?: boolean;
}

function CredentialInput({ label, value, onChange, placeholder, disabled = false }: CredentialInputProps) {
  return (
    <div>
      <label className="block text-xs font-semibold text-slate-300 mb-1.5">{label}</label>
      <input
        type="password"
        autoComplete="off"
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full rounded-md bg-slate-950/70 border border-slate-800 px-3 py-2 text-sm text-slate-100 font-mono focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:opacity-60"
      />
    </div>
  );
}
