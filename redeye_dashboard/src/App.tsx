import { Suspense, lazy, type ReactNode } from 'react';
import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { AuthProvider, useAuth } from './presentation/context/AuthContext';
import { ToastProvider } from './presentation/components/ui/ToastProvider';
import { useMetrics } from './presentation/hooks/useMetrics';

const LandingPage = lazy(() => import('./presentation/pages/LandingPage').then((module) => ({ default: module.LandingPage })));
const AuthPage = lazy(() => import('./presentation/pages/AuthPage').then((module) => ({ default: module.AuthPage })));
const OnboardingWizard = lazy(() => import('./presentation/pages/OnboardingWizard').then((module) => ({ default: module.OnboardingWizard })));
const DashboardLayout = lazy(() => import('./presentation/layouts/DashboardLayout').then((module) => ({ default: module.DashboardLayout })));
const DashboardView = lazy(() => import('./presentation/dashboard/DashboardView').then((module) => ({ default: module.DashboardView })));
const ComplianceView = lazy(() => import('./presentation/dashboard/ComplianceView').then((module) => ({ default: module.ComplianceView })));
const TracesView = lazy(() => import('./presentation/dashboard/TracesView').then((module) => ({ default: module.TracesView })));
const CacheView = lazy(() => import('./presentation/dashboard/CacheView').then((module) => ({ default: module.CacheView })));
const SettingsView = lazy(() => import('./presentation/dashboard/SettingsView').then((module) => ({ default: module.SettingsView })));

function DashboardIndex() {
  const { user } = useAuth();
  const { metrics, chartData, error, setError, calculateSavedCost } = useMetrics();
  const hasProviderConfigured = Boolean(
    user?.providerStatus.openaiConfigured
      || user?.providerStatus.anthropicConfigured
      || user?.providerStatus.geminiConfigured,
  );

  return (
    <DashboardView
      metrics={metrics}
      chartData={chartData}
      error={error}
      showProviderSetupNotice={!hasProviderConfigured}
      onErrorClear={() => setError(null)}
      calculateSavedCost={calculateSavedCost}
    />
  );
}

function RequireAuth({ children }: { children: ReactNode }) {
  const { isAuthenticated, isInitializing } = useAuth();
  const location = useLocation();

  if (isInitializing) {
    return null;
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  return <>{children}</>;
}

function RedirectIfAuth({ children }: { children: ReactNode }) {
  const { isAuthenticated, user, isInitializing } = useAuth();

  if (isInitializing) {
    return null;
  }

  if (isAuthenticated) {
    return <Navigate to={user?.onboardingComplete ? '/dashboard' : '/onboarding'} replace />;
  }

  return <>{children}</>;
}

function RouteLoader() {
  return (
    <div className="min-h-screen bg-slate-950 flex items-center justify-center px-6">
      <div className="rounded-2xl border border-slate-800 bg-slate-900/60 px-6 py-4 text-sm text-slate-300 shadow-2xl shadow-indigo-950/20">
        Loading workspace...
      </div>
    </div>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <AuthProvider>
        <BrowserRouter>
          <Suspense fallback={<RouteLoader />}>
            <Routes>
              <Route path="/" element={<LandingPage />} />

              <Route
                path="/login"
                element={
                  <RedirectIfAuth>
                    <AuthPage />
                  </RedirectIfAuth>
                }
              />

              <Route
                path="/onboarding"
                element={
                  <RequireAuth>
                    <OnboardingWizard />
                  </RequireAuth>
                }
              />

              <Route
                path="/dashboard"
                element={
                  <RequireAuth>
                    <DashboardLayout />
                  </RequireAuth>
                }
              >
                <Route index element={<DashboardIndex />} />
                <Route path="compliance" element={<ComplianceView />} />
                <Route path="traces" element={<TracesView />} />
                <Route path="cache" element={<CacheView />} />
                <Route path="settings" element={<SettingsView />} />
              </Route>

              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </Suspense>
        </BrowserRouter>
      </AuthProvider>
    </ToastProvider>
  );
}
