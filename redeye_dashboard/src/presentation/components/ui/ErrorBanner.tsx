import { AlertTriangle, X } from 'lucide-react';

interface ErrorBannerProps {
  error: string | null;
  onClose?: () => void;
  type?: 'error' | 'warning' | 'critical';
}

/**
 * RedEye Error Banner
 * Displays errors prominently with red highlighting
 */
export function ErrorBanner({
  error,
  onClose,
  type = 'error',
}: ErrorBannerProps) {
  if (!error) return null;

  const getStyles = () => {
    switch (type) {
      case 'critical':
        return {
          bg: 'bg-red-500/20',
          border: 'border-red-500',
          text: 'text-red-300',
          icon: 'text-red-400',
        };
      case 'warning':
        return {
          bg: 'bg-yellow-500/20',
          border: 'border-yellow-500',
          text: 'text-yellow-300',
          icon: 'text-yellow-400',
        };
      default:
        return {
          bg: 'bg-red-500/15',
          border: 'border-red-500/50',
          text: 'text-red-400',
          icon: 'text-red-500',
        };
    }
  };

  const styles = getStyles();

  return (
    <div
      className={`
        w-full px-4 py-3 rounded-lg border-l-4 flex items-start gap-3
        ${styles.bg} ${styles.border} backdrop-blur-md
        animate-in fade-in slide-in-from-top-2 duration-300
      `}
      role="alert"
    >
      <AlertTriangle className={`w-5 h-5 flex-shrink-0 mt-0.5 ${styles.icon}`} />
      <p className={`flex-1 text-sm font-medium ${styles.text}`}>
        {error}
      </p>
      {onClose && (
        <button
          onClick={onClose}
          className="flex-shrink-0 p-1 hover:bg-white/10 rounded transition-colors"
          aria-label="Close error"
        >
          <X className={`w-4 h-4 ${styles.icon}`} />
        </button>
      )}
    </div>
  );
}

export default ErrorBanner;