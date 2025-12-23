'use client';

import { useState, useEffect, useCallback } from 'react';

const API_BASE = typeof window !== 'undefined' 
  ? `${window.location.protocol}//${window.location.host}`
  : 'http://localhost:7777';

export interface TraceInfo {
  trace_id: string;
  process_name: string | null;
  started_at: string;
  duration_ms: number;
  total_tokens: number;
  llm_calls: number;
  tool_calls: number;
  is_complete: boolean;
}

export interface TracesResponse {
  traces: TraceInfo[];
  active: number;
  completed: number;
}

interface UseTracesReturn {
  traces: TracesResponse | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useTraces(): UseTracesReturn {
  const [traces, setTraces] = useState<TracesResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await fetch(`${API_BASE}/api/traces`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
      const data: TracesResponse = await response.json();
      setTraces(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch traces');
    } finally {
      setLoading(false);
    }
  }, []);
  
  useEffect(() => {
    refresh();
    
    // Refresh every 5 seconds
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, [refresh]);
  
  return { traces, loading, error, refresh };
}

