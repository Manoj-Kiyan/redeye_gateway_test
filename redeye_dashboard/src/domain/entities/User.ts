// Domain Entity - User
// Represents an authenticated workspace operator.

export interface ProviderCredentialStatus {
  openaiConfigured: boolean;
  anthropicConfigured: boolean;
  geminiConfigured: boolean;
}

export interface User {
  id: string;
  email: string;
  role: 'owner' | 'admin' | 'viewer';
  workspaceName: string;
  openAiApiKey: string;
  onboardingComplete: boolean;
  redeyeApiKey?: string;
  providerStatus: ProviderCredentialStatus;
}
