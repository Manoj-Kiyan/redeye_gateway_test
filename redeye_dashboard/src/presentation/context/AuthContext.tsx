// src/presentation/context/AuthContext.tsx
import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import type { User } from '../../domain/entities/User';
import type { OnboardingPayload } from '../../domain/usecases/AuthUseCase';
import { authService } from '../../data/services/authService';

interface AuthContextValue {
  isAuthenticated: boolean;
  isInitializing: boolean;
  user: User | null;
  login(email: string, password: string): Promise<void>;
  signup(email: string, password: string, companyName: string): Promise<void>;
  completeOnboarding(payload: OnboardingPayload): Promise<void>;
  logout(): void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [isInitializing, setIsInitializing] = useState(true);

  useEffect(() => {
    const initAuth = async () => {
      const token = localStorage.getItem('re_token');

      if (token) {
        const refreshedUser = await authService.refreshToken();
        if (refreshedUser) {
          setUser(refreshedUser);
        } else {
          localStorage.removeItem('re_token');
        }
      }

      setIsInitializing(false);
    };

    void initAuth();
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    const nextUser = await authService.login({ email, password });
    setUser(nextUser);
  }, []);

  const signup = useCallback(async (email: string, password: string, companyName: string) => {
    const nextUser = await authService.signup({ email, password, companyName });
    setUser(nextUser);
  }, []);

  const completeOnboarding = useCallback(async (payload: OnboardingPayload) => {
    if (!user) {
      throw new Error('Not authenticated');
    }

    const updatedUser = await authService.completeOnboarding(user.id, payload);
    setUser(updatedUser);
  }, [user]);

  const logout = useCallback(() => {
    localStorage.removeItem('re_token');
    setUser(null);
  }, []);

  return (
    <AuthContext.Provider value={{ isAuthenticated: user !== null, isInitializing, user, login, signup, completeOnboarding, logout }}>
      {!isInitializing && children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);

  if (!ctx) {
    throw new Error('useAuth must be used inside <AuthProvider>');
  }

  return ctx;
}
