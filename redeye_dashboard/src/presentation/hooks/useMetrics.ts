// src/presentation/hooks/useMetrics.ts
import { useState, useEffect, useRef } from 'react';
import { useToast } from '../components/ui/ToastProvider';

export interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
}

export function useMetrics() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [chartData, setChartData] = useState<{ time: string; requests: number; latency: number }[]>([]);
  const [error, setError] = useState<string | null>(null);
  const { addToast } = useToast();
  
  // Prevent toast spamming
  const isErrorShowingRef = useRef(false);

  useEffect(() => {
    let alive = true;
    const abortController = new AbortController();

    const fetchMetrics = async () => {
      try {
        const res = await fetch('http://localhost:8080/v1/admin/metrics', {
          signal: abortController.signal
        });
        
        if (!res.ok) throw new Error(`HTTP ${res.status}: Failed to fetch metrics`);
        
        const data: Metrics = await res.json();
        if (!alive) return;
        
        setMetrics(data);
        setError(null);
        isErrorShowingRef.current = false; // Reset on success

        const now = new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
        
        setChartData((prev) => [...prev, {
          time: now,
          requests: parseInt(data.total_requests || '0', 10),
          latency: Math.round(data.avg_latency_ms || 0),
        }].slice(-10));

      } catch (err: unknown) {
        // Ignore abort errors on unmount
        if (!alive || (err instanceof Error && err.name === 'AbortError')) return;
        
        const errorMsg = err instanceof Error ? err.message : 'Unknown error fetching metrics';
        setError(errorMsg);
        
        // Show toast ONLY once per failure cycle
        if (!isErrorShowingRef.current) {
          isErrorShowingRef.current = true;
          addToast({ type: 'error', message: errorMsg, duration: 5000 });
        }
      }
    };

    fetchMetrics();
    const id = setInterval(fetchMetrics, 3000);
    
    return () => { 
      alive = false; 
      clearInterval(id); 
      abortController.abort(); // Cleanup network requests
    };
  }, [addToast]);

  const calculateSavedCost = () => {
    if (!metrics?.rate_limited_requests) return '0.00';
    const rateLimited = parseInt(metrics.rate_limited_requests, 10);
    return isNaN(rateLimited) ? '0.00' : (rateLimited * 0.005).toFixed(2);
  };

  return { metrics, chartData, error, setError, calculateSavedCost };
}