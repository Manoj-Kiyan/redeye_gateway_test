// Data Service - Auth API calls to RedEye Gateway

import type { User } from '../../domain/entities/User';
import type {
  IAuthUseCase,
  LoginPayload,
  OnboardingPayload,
  SignupPayload,
} from '../../domain/usecases/AuthUseCase';

export interface ProviderStatus {
  openaiConfigured: boolean;
  anthropicConfigured: boolean;
  geminiConfigured: boolean;
  redeyeApiKey?: string;
  workspaceName: string;
}

export interface TenantRoute {
  provider: 'openai' | 'anthropic' | 'gemini';
  model: string;
  isDefault: boolean;
}

export interface ProviderCatalogEntry {
  provider: TenantRoute['provider'];
  suggestedModels: string[];
}

export interface AuditLogEntry {
  id: string;
  service: 'auth' | 'gateway';
  action: string;
  targetType: string;
  metadata: Record<string, unknown>;
  actorUserId?: string;
  createdAt: string;
}

interface ProviderStatusResponse {
  openai_configured: boolean;
  anthropic_configured: boolean;
  gemini_configured: boolean;
  redeye_api_key?: string;
  workspace_name: string;
}

interface TenantRoutesResponse {
  tenant_id: string;
  routes: TenantRoute[];
}

interface ProviderCatalogResponse {
  providers: Array<{
    provider: TenantRoute['provider'];
    suggested_models: string[];
  }>;
}

interface AuditLogResponse {
  tenant_id: string;
  entries: Array<{
    id: string;
    service: 'auth' | 'gateway';
    action: string;
    target_type: string;
    metadata: Record<string, unknown>;
    actor_user_id?: string;
    created_at: string;
  }>;
}

export interface IAuthUseCaseExtended extends IAuthUseCase {
  refreshToken(): Promise<User | null>;
  getProviderStatus(): Promise<ProviderStatus>;
  updateProviderCredentials(payload: {
    openAiApiKey?: string;
    anthropicApiKey?: string;
    geminiApiKey?: string;
  }): Promise<ProviderStatus>;
  getProviderCatalog(): Promise<ProviderCatalogEntry[]>;
  getTenantRoutes(): Promise<TenantRoute[]>;
  updateTenantRoutes(routes: TenantRoute[]): Promise<TenantRoute[]>;
  getAuditLogs(): Promise<AuditLogEntry[]>;
}

const AUTH_BASE_URL = 'http://localhost:8084/v1/auth';
const GATEWAY_BASE_URL = 'http://localhost:8080/v1/admin';

interface AuthResponse {
  id: string;
  email: string;
  tenant_id: string;
  workspace_name: string;
  onboarding_complete: boolean;
  token: string;
  redeye_api_key?: string;
}

function getToken() {
  return localStorage.getItem('re_token') || '';
}

function mapProviderStatus(resp: ProviderStatusResponse): ProviderStatus {
  return {
    openaiConfigured: resp.openai_configured,
    anthropicConfigured: resp.anthropic_configured,
    geminiConfigured: resp.gemini_configured,
    redeyeApiKey: resp.redeye_api_key ?? undefined,
    workspaceName: resp.workspace_name,
  };
}

function mapUser(resp: AuthResponse): User {
  return {
    id: resp.id,
    email: resp.email,
    workspaceName: resp.workspace_name ?? '',
    openAiApiKey: '',
    onboardingComplete: resp.onboarding_complete ?? false,
    redeyeApiKey: resp.redeye_api_key ?? undefined,
    providerStatus: {
      openaiConfigured: resp.onboarding_complete ?? false,
      anthropicConfigured: false,
      geminiConfigured: false,
    },
  };
}

async function requestJson<T>(url: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers ?? {});
  if (!headers.has('Content-Type') && options.body) {
    headers.set('Content-Type', 'application/json');
  }

  const res = await fetch(url, {
    ...options,
    headers,
  });

  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    try {
      const parsed = JSON.parse(text) as { error?: string };
      throw new Error(parsed.error || text || `HTTP ${res.status}`);
    } catch {
      throw new Error(text || `HTTP ${res.status}`);
    }
  }

  return res.json() as Promise<T>;
}

export const authService: IAuthUseCaseExtended = {
  async login({ email, password }: LoginPayload): Promise<User> {
    const data = await requestJson<AuthResponse>(`${AUTH_BASE_URL}/login`, {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async signup({ email, password, companyName }: SignupPayload): Promise<User> {
    const data = await requestJson<AuthResponse>(`${AUTH_BASE_URL}/signup`, {
      method: 'POST',
      body: JSON.stringify({
        email,
        password,
        company_name: companyName,
      }),
    });
    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async completeOnboarding(_userId: string, payload: OnboardingPayload): Promise<User> {
    const data = await requestJson<AuthResponse>(`${AUTH_BASE_URL}/onboard`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
      body: JSON.stringify({
        openai_api_key: payload.openAiApiKey,
        anthropic_api_key: payload.anthropicApiKey || undefined,
        gemini_api_key: payload.geminiApiKey || undefined,
        workspace_name: payload.workspaceName,
      }),
    });

    if (data.token) {
      localStorage.setItem('re_token', data.token);
    }
    return mapUser(data);
  },

  async refreshToken(): Promise<User | null> {
    try {
      const res = await fetch(`${AUTH_BASE_URL}/refresh`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
      });

      if (!res.ok) {
        return null;
      }

      const data = await res.json() as AuthResponse;
      if (data.token) {
        localStorage.setItem('re_token', data.token);
        return mapUser(data);
      }

      return null;
    } catch {
      return null;
    }
  },

  async getProviderStatus(): Promise<ProviderStatus> {
    const data = await requestJson<ProviderStatusResponse>(`${AUTH_BASE_URL}/providers`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
    });

    return mapProviderStatus(data);
  },

  async updateProviderCredentials(payload: { openAiApiKey?: string; anthropicApiKey?: string; geminiApiKey?: string; }): Promise<ProviderStatus> {
    const data = await requestJson<ProviderStatusResponse>(`${AUTH_BASE_URL}/providers`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
      body: JSON.stringify({
        openai_api_key: payload.openAiApiKey || undefined,
        anthropic_api_key: payload.anthropicApiKey || undefined,
        gemini_api_key: payload.geminiApiKey || undefined,
      }),
    });

    return mapProviderStatus(data);
  },

  async getTenantRoutes(): Promise<TenantRoute[]> {
    const data = await requestJson<TenantRoutesResponse>(`${GATEWAY_BASE_URL}/routes`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
    });

    return data.routes;
  },

  async getProviderCatalog(): Promise<ProviderCatalogEntry[]> {
    const data = await requestJson<ProviderCatalogResponse>(`${GATEWAY_BASE_URL}/catalog`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
    });

    return data.providers.map((provider) => ({
      provider: provider.provider,
      suggestedModels: provider.suggested_models,
    }));
  },

  async updateTenantRoutes(routes: TenantRoute[]): Promise<TenantRoute[]> {
    const data = await requestJson<TenantRoutesResponse>(`${GATEWAY_BASE_URL}/routes`, {
      method: 'PUT',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
      body: JSON.stringify({ routes }),
    });

    return data.routes;
  },

  async getAuditLogs(): Promise<AuditLogEntry[]> {
    const data = await requestJson<AuditLogResponse>(`${GATEWAY_BASE_URL}/audit`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${getToken()}`,
      },
    });

    return data.entries.map((entry) => ({
      id: entry.id,
      service: entry.service,
      action: entry.action,
      targetType: entry.target_type,
      metadata: entry.metadata,
      actorUserId: entry.actor_user_id,
      createdAt: entry.created_at,
    }));
  },
};
