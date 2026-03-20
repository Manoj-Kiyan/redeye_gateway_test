import { useCallback, useMemo, useState, createContext, useContext, type ReactNode, type FC } from 'react';
import { AlertCircle, AlertTriangle, X } from 'lucide-react';

export interface Toast {
  id: string;
  message: string;
  type: 'error' | 'warning' | 'success' | 'info';
  duration?: number;
  action?: { label: string; onClick: () => void };
}

const ToastContext = createContext<{
  toasts: Toast[];
  addToast: (toast: Omit<Toast, 'id'>) => string;
  removeToast: (id: string) => void;
  clearAll: () => void;
} | null>(null);

export const ToastProvider: FC<{ children: ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id));
  }, []);

  const addToast = useCallback((toast: Omit<Toast, 'id'>) => {
    const id = `${Date.now()}-${Math.random()}`;
    const duration = toast.duration ?? 5000;
    const fullToast: Toast = { ...toast, id, duration };

    setToasts((prev) => [...prev, fullToast]);

    if (duration > 0) {
      window.setTimeout(() => removeToast(id), duration);
    }

    return id;
  }, [removeToast]);

  const clearAll = useCallback(() => {
    setToasts([]);
  }, []);

  const value = useMemo(() => ({ toasts, addToast, removeToast, clearAll }), [toasts, addToast, removeToast, clearAll]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <ToastContainer toasts={toasts} onRemove={removeToast} />
    </ToastContext.Provider>
  );
};

export const useToast = () => {
  const context = useContext(ToastContext);

  if (!context) {
    throw new Error('useToast must be used within ToastProvider');
  }

  return context;
};

interface ToastContainerProps {
  toasts: Toast[];
  onRemove: (id: string) => void;
}

const ToastContainer: FC<ToastContainerProps> = ({ toasts, onRemove }) => {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex max-w-sm flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onRemove={onRemove} />
      ))}
    </div>
  );
};

interface ToastItemProps {
  toast: Toast;
  onRemove: (id: string) => void;
}

const ToastItem: FC<ToastItemProps> = ({ toast, onRemove }) => {
  const styles = useMemo(() => {
    switch (toast.type) {
      case 'error':
        return {
          bg: 'bg-red-600',
          border: 'border-red-500/30',
          icon: AlertCircle,
          textColor: 'text-white',
        };
      case 'warning':
        return {
          bg: 'bg-yellow-600',
          border: 'border-yellow-500/30',
          icon: AlertTriangle,
          textColor: 'text-white',
        };
      case 'success':
        return {
          bg: 'bg-emerald-600',
          border: 'border-emerald-500/30',
          icon: AlertCircle,
          textColor: 'text-white',
        };
      default:
        return {
          bg: 'bg-slate-700',
          border: 'border-slate-600/30',
          icon: AlertCircle,
          textColor: 'text-white',
        };
    }
  }, [toast.type]);

  const { bg, border, icon: IconComponent, textColor } = styles;

  return (
    <div
      className={`
        ${bg} ${border} border rounded-lg px-4 py-3 flex gap-3 items-start
        backdrop-blur-md shadow-lg pointer-events-auto
        animate-in fade-in slide-in-from-bottom-4 duration-300
      `}
      role="alert"
    >
      <IconComponent className={`w-5 h-5 flex-shrink-0 mt-0.5 ${textColor}`} />

      <div className="flex-1 min-w-0">
        <p className={`text-sm font-medium ${textColor} break-words`}>
          {toast.message}
        </p>
        {toast.action && (
          <button
            onClick={toast.action.onClick}
            className={`text-xs mt-2 font-semibold underline ${textColor} hover:opacity-80 transition-opacity`}
          >
            {toast.action.label}
          </button>
        )}
      </div>

      <button
        onClick={() => onRemove(toast.id)}
        className={`flex-shrink-0 p-1 hover:bg-white/20 rounded transition-colors ${textColor}`}
        aria-label="Close notification"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
};

export default ToastProvider;