# RedEye Dashboard Error Display Implementation

## Overview
Comprehensive red error visualization system added to the RedEye AI Engine dashboard. Provides real-time error alerts, toast notifications, and persistent error logging across all dashboard views.

## Components Created

### 1. **ErrorBanner.tsx**
- **Location:** `redeye_dashboard/src/presentation/components/ErrorBanner.tsx`
- **Purpose:** Prominent alert banner for displaying errors at the top of views
- **Features:**
  - Red color variants (error, warning, critical)
  - Dismissible with X button
  - Alert triangle icon for visual emphasis
  - Slide-in animation from top
  - Backdrop blur effect
  - Full width spanning container

**Props:**
```typescript
interface ErrorBannerProps {
  error: string | null;
  type?: 'error' | 'warning' | 'critical';
  onClose?: () => void;
}
```

**Styling:**
- Error: `bg-red-500/10 border-l-4 border-red-500 text-red-400`
- Warning: `bg-yellow-500/10 border-l-4 border-yellow-500 text-yellow-400`
- Critical: `bg-red-600/20 border-l-4 border-red-600 text-red-300`

### 2. **ToastProvider.tsx**
- **Location:** `redeye_dashboard/src/presentation/components/ToastProvider.tsx`
- **Purpose:** Global notification system for transient error/success messages
- **Features:**
  - React Context for global state
  - `useToast()` hook for component access
  - Auto-dismiss with configurable duration
  - Bottom-right positioning
  - Multiple toast variants (error, warning, success, info)

**Usage:**
```typescript
const { addToast } = useToast();

addToast({
  type: 'error',
  message: 'Something went wrong',
  duration: 5000,
});
```

**Styling:**
- Error toasts: Red with `bg-red-600/90`
- Warning toasts: Yellow with `bg-yellow-600/90`
- Success toasts: Green with `bg-emerald-600/90`
- Info toasts: Slate with `bg-slate-700/90`

### 3. **ErrorLog.tsx**
- **Location:** `redeye_dashboard/src/presentation/components/ErrorLog.tsx`
- **Purpose:** Persistent error history table for debugging
- **Features:**
  - Expandable error entries
  - Stack trace display
  - Copy-to-clipboard functionality
  - Severity color coding
  - Sortable by timestamp
  - Clear all errors button

**Props:**
```typescript
interface ErrorLogProps {
  errors: ErrorLogEntry[];
  onClear?: () => void;
  maxHeight?: string;
}

interface ErrorLogEntry {
  id: string;
  timestamp: Date;
  message: string;
  service?: string;
  code?: string;
  severity: 'error' | 'warning' | 'critical';
  stackTrace?: string;
}
```

## Integration Points

### 1. **App.tsx** (Main Router)
- Wrapped entire app with `<ToastProvider>`
- Ensures `useToast()` hook available throughout all routes
- Global error notification capability

### 2. **AuthPage.tsx** (Login/Signup)
```typescript
// Before:
{error && (
  <p className="text-xs text-rose-400 bg-rose-500/10 ...">
    {error}
  </p>
)}

// After:
{error && (
  <ErrorBanner error={error} type="error" onClose={() => setError(null)} />
)}
```

### 3. **OnboardingWizard.tsx** (Setup)
- Same ErrorBanner replacement for consistency
- Dismissible error display during workspace and API key setup

### 4. **DashboardIndex.tsx** (Dashboard Main)
```typescript
const { addToast } = useToast();

// On fetch error:
addToast({
  type: 'error',
  message: errorMsg,
  duration: 5000,
});
```

### 5. **DashboardView.tsx** (Metrics Display)
- Added `onErrorClear` prop for error dismissal
- ErrorBanner displays at top of view
- Real-time error indicator (red dot in header)

## Color Theme
All error components use **red** as primary error indicator to match user request "show err red lin":

| Severity | Background | Border | Text |
|----------|-----------|--------|------|
| Error | `red-500/10` | `red-500` | `red-400` |
| Warning | `yellow-500/10` | `yellow-500` | `yellow-400` |
| Critical | `red-600/20` | `red-600` | `red-300` |

## User Experience

### Error Flow
1. **API Call Fails** → Captured in catch block
2. **Toast Notification** → Bottom-right auto-dismiss alert (5s default)
3. **ErrorBanner** → Persistent alert at top of view
4. **User Dismissal** → Click X button to close banner
5. **Error History** → Available in ErrorLog if integrated

### Visual Hierarchy
- **Most Prominent:** ErrorBanner (full width, top position)
- **Secondary:** Toast notifications (corner, auto-dismiss)
- **Reference:** ErrorLog table (for debugging)

## Files Modified

| File | Changes |
|------|---------|
| `App.tsx` | Added ToastProvider wrapper, updated DashboardView props |
| `AuthPage.tsx` | Replaced inline error with ErrorBanner |
| `OnboardingWizard.tsx` | Replaced inline error with ErrorBanner |
| `DashboardView.tsx` | Added ErrorBanner at top, new onErrorClear prop |

## Implementation Status

### ✅ Complete
- ErrorBanner component creation
- ToastProvider context system
- ErrorLog component with copy functionality
- Integration with AuthPage
- Integration with OnboardingWizard
- Integration with DashboardIndex for toast notifications
- Integration with DashboardView for banner display
- App-level ToastProvider wrapper

### 🔄 Optional Enhancements (Future)
- Connect ErrorLog to actual error tracking in DashboardView
- Add error correlation IDs from backend
- Implement persistent error storage (IndexedDB)
- Add retry buttons to error toasts
- Create error analytics dashboard
- Add sentry-style error capturing

## Testing

### Manual Testing Checklist
- [ ] AuthPage: Login failure shows red ErrorBanner and toast
- [ ] OnboardingWizard: API key errors display red banner
- [ ] DashboardIndex: Metrics fetch failure shows toast + banner
- [ ] Toast auto-dismisses after 5 seconds
- [ ] Clicking X closes ErrorBanner
- [ ] Multiple toasts stack vertically
- [ ] Error text wraps properly in mobile view

### Responsive Design
- Mobile: Stack toasts vertically, full-width banners
- Desktop: Bottom-right toast grouping, normal widths
- Tablet: Responsive scaling

## Tailwind CSS Classes Used
- `backdrop-blur-md`: Glass effect on components
- `animate-in`, `fade-in`, `slide-in-from-top-2`: Entrance animations
- `z-50`: Error components above other content
- Color utilities: `red-500`, `red-400`, `red-600`, etc.
- Spacing: Consistent `px-4 py-3` padding for alerts

## Browser Compatibility
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers (iOS Safari, Chrome Mobile)

## Future Integration Points

### With Rust Services
- Correlate client errors with backend error codes
- Display service name and endpoint in ErrorLog
- Include trace IDs from headers

### With ClickHouse Logging
- Store error events for audit trail
- Query error patterns and frequency
- Create error alerting rules

### With OpenAI Integration
- Display LLM-specific errors (rate limits, invalid keys, etc.)
- Provide human-readable error messages
- Suggest remediation steps
