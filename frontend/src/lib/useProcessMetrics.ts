'use client';

import { useState, useEffect, useCallback } from 'react';

const API_BASE = typeof window !== 'undefined' 
  ? `${window.location.protocol}//${window.location.host}`
  : 'http://localhost:7777';

export interface ProcessResourceInfo {
  pid: number;
  comm: string;
  cpu_percent: number;
  memory_rss_mb: number;
  memory_vms_mb: number;
}

export interface ProcessMetricsResponse {
  processes: ProcessResourceInfo[];
}

interface UseProcessMetricsReturn {
  metrics: ProcessResourceInfo[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useProcessMetrics(refreshInterval = 3000): UseProcessMetricsReturn {
  const [metrics, setMetrics] = useState<ProcessResourceInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  const refresh = useCallback(async () => {
    try {
      setError(null);
      
      const response = await fetch(`${API_BASE}/api/metrics/processes`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
      const data: ProcessMetricsResponse = await response.json();
      setMetrics(data.processes);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch process metrics');
    } finally {
      setLoading(false);
    }
  }, []);
  
  useEffect(() => {
    refresh();
    
    // Refresh at the specified interval
    const interval = setInterval(refresh, refreshInterval);
    return () => clearInterval(interval);
  }, [refresh, refreshInterval]);
  
  return { metrics, loading, error, refresh };
}

