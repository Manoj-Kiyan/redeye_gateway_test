// Presentation Page - OnboardingWizard
// Step 1: Workspace name | Step 2: Provider credentials

import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { ErrorBanner } from '../components/ui/ErrorBanner';
import { Loader2, ChevronRight, KeyRound, Building2, ShieldCheck } from 'lucide-react';

type Step = 1 | 2;

export function OnboardingWizard() {
  const navigate = useNavigate();
  const { completeOnboarding } = useAuth();

  const [step, setStep] = useState<Step>(1);
  const [workspaceName, setWorkspaceName] = useState('');
  const [openAiApiKey, setOpenAiApiKey] = useState('');
  const [anthropicApiKey, setAnthropicApiKey] = useState('');
  const [geminiApiKey, setGeminiApiKey] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  function handleStep1(e: FormEvent) {
    e.preventDefault();
    if (!workspaceName.trim()) return;
    setStep(2);
  }

  async function handleFinish(e: FormEvent) {
    e.preventDefault();
    if (!openAiApiKey.trim()) {
      setError('OpenAI API key is required to complete onboarding.');
      return;
    }

    setError(null);
    setLoading(true);
    try {
      await completeOnboarding({
        workspaceName: workspaceName.trim(),
        openAiApiKey: openAiApiKey.trim(),
        anthropicApiKey: anthropicApiKey.trim() || undefined,
        geminiApiKey: geminiApiKey.trim() || undefined,
      });
      navigate('/dashboard');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Something went wrong.');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-slate-950 flex items-center justify-center px-4">
      <div className="w-full max-w-2xl">
        <div className="flex items-center justify-between mb-10">
          <div className="flex items-center gap-2.5">
            <div className="h-8 w-8 rounded-xl bg-indigo-600 flex items-center justify-center shadow-[0_0_20px_rgba(99,102,241,0.5)]">
              <span className="text-xs font-bold text-white">RE</span>
            </div>
            <span className="text-sm font-semibold text-slate-100">RedEye</span>
          </div>
          <div className="flex items-center gap-2">
            {([1, 2] as Step[]).map((s) => (
              <div
                key={s}
                className={`h-2 w-2 rounded-full transition-colors ${
                  s === step ? 'bg-indigo-500' : s < step ? 'bg-indigo-500/40' : 'bg-slate-700'
                }`}
              />
            ))}
          </div>
        </div>

        {step === 1 && (
          <div className="glass-panel bg-slate-900/50 border border-slate-800 p-8">
            <Building2 className="w-7 h-7 text-indigo-400 mb-4" />
            <h1 className="text-xl font-bold text-slate-50 mb-1">Name your workspace</h1>
            <p className="text-sm text-slate-400 mb-7">
              This appears in your dashboard, tracing metadata, and provider routing policies.
            </p>
            <form onSubmit={handleStep1} className="space-y-4">
              <input
                type="text"
                required
                autoFocus
                value={workspaceName}
                onChange={(e) => setWorkspaceName(e.target.value)}
                placeholder="e.g. Acme AI Platform"
                className="w-full rounded-lg bg-slate-950/70 border border-slate-800 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-600 focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 transition-colors"
              />
              <button
                type="submit"
                className="w-full inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 px-4 py-2.5 text-sm font-semibold text-white shadow-[0_0_20px_rgba(99,102,241,0.25)] transition-all duration-200"
              >
                Continue <ChevronRight className="w-4 h-4" />
              </button>
            </form>
          </div>
        )}

        {step === 2 && (
          <div className="glass-panel bg-slate-900/50 border border-slate-800 p-8 space-y-6">
            <div>
              <KeyRound className="w-7 h-7 text-indigo-400 mb-4" />
              <h1 className="text-xl font-bold text-slate-50 mb-1">Connect provider credentials</h1>
              <p className="text-sm text-slate-400">
                OpenAI is required today. Anthropic and Gemini are optional, but adding them now unlocks multi-provider routing later.
              </p>
            </div>

            <form onSubmit={handleFinish} className="space-y-4">
              <CredentialField
                label="OpenAI API Key"
                required
                value={openAiApiKey}
                onChange={setOpenAiApiKey}
                placeholder="sk-..."
                helper="Required for onboarding and current default gateway path."
              />
              <CredentialField
                label="Anthropic API Key"
                value={anthropicApiKey}
                onChange={setAnthropicApiKey}
                placeholder="sk-ant-..."
                helper="Optional. Used when tenant routes target Claude models."
              />
              <CredentialField
                label="Gemini API Key"
                value={geminiApiKey}
                onChange={setGeminiApiKey}
                placeholder="AIza..."
                helper="Optional. Used when tenant routes target Gemini models."
              />

              <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-4 py-3 text-sm text-emerald-200 flex gap-3 items-start">
                <ShieldCheck className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <p>Provider keys are encrypted before storage. Raw secrets are never shown again after submission.</p>
              </div>

              {error && <ErrorBanner error={error} type="error" onClose={() => setError(null)} />}

              <div className="flex gap-3">
                <button
                  type="button"
                  onClick={() => setStep(1)}
                  className="flex-none rounded-lg border border-slate-700 bg-slate-900/50 px-4 py-2.5 text-sm font-semibold text-slate-400 hover:text-slate-200 transition-colors"
                >
                  Back
                </button>
                <button
                  type="submit"
                  disabled={loading}
                  className="flex-1 inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-60 disabled:cursor-not-allowed px-4 py-2.5 text-sm font-semibold text-white shadow-[0_0_20px_rgba(99,102,241,0.25)] transition-all duration-200"
                >
                  {loading && <Loader2 className="w-4 h-4 animate-spin" />}
                  Finish setup
                </button>
              </div>
            </form>
          </div>
        )}

        <p className="mt-5 text-center text-xs text-slate-600">Step {step} of 2</p>
      </div>
    </div>
  );
}

interface CredentialFieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  helper: string;
  required?: boolean;
}

function CredentialField({ label, value, onChange, placeholder, helper, required = false }: CredentialFieldProps) {
  return (
    <div>
      <label className="block text-sm font-semibold text-slate-200 mb-1.5">{label}</label>
      <input
        type="password"
        required={required}
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full rounded-lg bg-slate-950/70 border border-slate-800 px-3 py-2.5 text-sm text-slate-100 font-mono placeholder:text-slate-600 focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 transition-colors"
      />
      <p className="mt-1.5 text-xs text-slate-500">{helper}</p>
    </div>
  );
}
