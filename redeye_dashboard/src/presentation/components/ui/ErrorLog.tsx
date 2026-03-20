import { useState, type FC } from 'react';
import { AlertCircle, Trash2, Copy } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';

export interface ErrorLogEntry {
  id: string;
  timestamp: Date;
  message: string;
  service: string;
  code?: string;
  severity: 'error' | 'warning' | 'critical';
  stackTrace?: string;
}

interface ErrorLogProps {
  errors: ErrorLogEntry[];
  onClear?: () => void;
  maxHeight?: string;
}

/**
 * RedEye Error Log Component
 * Displays all errors with red highlighting in a scrollable table
 */
export const ErrorLog: FC<ErrorLogProps> = ({
  errors,
  onClear,
  maxHeight = 'max-h-96',
}) => {
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical':
        return 'bg-red-600/40 border-l-red-600 text-red-300';
      case 'error':
        return 'bg-red-500/30 border-l-red-500 text-red-200';
      case 'warning':
        return 'bg-yellow-500/20 border-l-yellow-500 text-yellow-200';
      default:
        return 'bg-slate-700/20 border-l-slate-500 text-slate-200';
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      alert('Copied to clipboard!');
    });
  };

  if (errors.length === 0) {
    return (
      <div className="rounded-lg border border-slate-700/50 bg-slate-900/30 p-4 text-center text-slate-400">
        <AlertCircle className="w-5 h-5 mx-auto mb-2 opacity-50" />
        <p className="text-sm">No errors logged</p>
      </div>
    );
  }

  return (
    <div className={`${maxHeight} flex flex-col rounded-lg border border-slate-700/50 bg-slate-900/20 overflow-hidden backdrop-blur-sm`}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-slate-700/30 bg-slate-900/40">
        <h3 className="text-sm font-semibold text-slate-200 flex items-center gap-2">
          <AlertCircle className="w-4 h-4 text-red-400" />
          Error Log ({errors.length})
        </h3>
        {onClear && errors.length > 0 && (
          <button
            onClick={onClear}
            className="p-1.5 hover:bg-red-500/20 rounded transition-colors text-red-400 hover:text-red-300"
            title="Clear error log"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Error List */}
      <div className="flex-1 overflow-y-auto custom-scrollbar">
        {errors.map((error) => (
          <div
            key={error.id}
            className={`border-l-4 px-4 py-3 cursor-pointer hover:bg-slate-800/30 transition-colors ${getSeverityColor(
              error.severity
            )}`}
            onClick={() =>
              setExpandedId(expandedId === error.id ? null : error.id)
            }
          >
            {/* Error Header Row */}
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="inline-block px-2 py-0.5 text-xs font-mono rounded bg-slate-800/50 text-slate-300">
                    {error.service}
                  </span>
                  {error.code && (
                    <span className="text-xs text-slate-400">Code: {error.code}</span>
                  )}
                  <span className="text-xs text-slate-500">
                    {formatDistanceToNow(error.timestamp, { addSuffix: true })}
                  </span>
                </div>
                <p className="text-sm font-medium text-white break-words">
                  {error.message}
                </p>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleCopy(error.message);
                }}
                className="flex-shrink-0 p-1.5 hover:bg-white/10 rounded transition-colors"
                title="Copy error message"
              >
                <Copy className="w-4 h-4 text-slate-400 hover:text-slate-200" />
              </button>
            </div>

            {/* Expanded Stack Trace */}
            {expandedId === error.id && error.stackTrace && (
              <pre className="mt-3 p-2 text-xs bg-slate-950/60 rounded border border-slate-700/30 text-slate-300 overflow-x-auto max-h-48">
                {error.stackTrace}
              </pre>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default ErrorLog;
