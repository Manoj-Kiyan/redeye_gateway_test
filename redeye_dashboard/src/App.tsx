import type { ReactNode } from 'react';
import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { AuthProvider, useAuth } from './presentation/context/AuthContext';
import { ToastProvider } from './presentation/components/ui/ToastProvider';
import { DashboardView } from './presentation/dashboard/DashboardView';
import { CacheView } from './presentation/dashboard/CacheView';
import { ComplianceView } from './presentation/dashboard/ComplianceView';
import { SettingsView } from './presentation/dashboard/SettingsView';
import { TracesView } from './presentation/dashboard/TracesView';
import { useMetrics } from './presentation/hooks/useMetrics';
import { DashboardLayout } from './presentation/layouts/DashboardLayout';
import { AuthPage } from './presentation/pages/AuthPage';
import { LandingPage } from './presentation/pages/LandingPage';
import { OnboardingWizard } from './presentation/pages/OnboardingWizard';

function DashboardIndex() {
  const { metrics, chartData, error, setError, calculateSavedCost } = useMetrics();

  return (
    <DashboardView
      metrics={metrics}
      chartData={chartData}
      error={error}
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

export default function App() {
  return (
    <ToastProvider>
      <AuthProvider>
        <BrowserRouter>
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
        </BrowserRouter>
      </AuthProvider>
    </ToastProvider>
  );
}