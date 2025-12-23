'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { WebEvent, WebEventsResponse } from '@/types/event';

const API_BASE = typeof window !== 'undefined' 
  ? `${window.location.protocol}//${window.location.host}`
  : 'http://localhost:7777';

const WS_URL = typeof window !== 'undefined'
  ? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`
  : 'ws://localhost:7777/ws';

interface UseEventsOptions {
  /** Enable real-time updates via WebSocket */
  realtime?: boolean;
  /** Maximum events to keep in memory */
  maxEvents?: number;
}

interface UseEventsReturn {
  /** List of events */
  events: WebEvent[];
  /** Whether initial load is in progress */
  loading: boolean;
  /** Error message if any */
  error: string | null;
  /** WebSocket connection status */
  connected: boolean;
  /** Manually refresh events from API */
  refresh: () => Promise<void>;
  /** Clear all events */
  clear: () => void;
}

/**
 * Hook to fetch and subscribe to events
 */
export function useEvents(options: UseEventsOptions = {}): UseEventsReturn {
  const { realtime = true, maxEvents = 1000 } = options;
  
  const [events, setEvents] = useState<WebEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  // Fetch initial events from API
  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await fetch(`${API_BASE}/api/web-events`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
      const data: WebEventsResponse = await response.json();
      setEvents(data.events);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch events');
    } finally {
      setLoading(false);
    }
  }, []);
  
  // Clear all events
  const clear = useCallback(() => {
    setEvents([]);
  }, []);
  
  // Connect to WebSocket for real-time updates
  const connectWebSocket = useCallback(() => {
    if (!realtime) return;
    
    try {
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;
      
      ws.onopen = () => {
        setConnected(true);
        setError(null);
      };
      
      ws.onmessage = (event) => {
        try {
          const webEvent: WebEvent = JSON.parse(event.data);
          setEvents((prev) => {
            const updated = [webEvent, ...prev];
            // Trim to max events
            if (updated.length > maxEvents) {
              return updated.slice(0, maxEvents);
            }
            return updated;
          });
        } catch (err) {
          console.error('Failed to parse WebSocket message:', err);
        }
      };
      
      ws.onerror = () => {
        setError('WebSocket connection error');
      };
      
      ws.onclose = () => {
        setConnected(false);
        wsRef.current = null;
        
        // Attempt to reconnect after 3 seconds
        reconnectTimeoutRef.current = setTimeout(() => {
          connectWebSocket();
        }, 3000);
      };
    } catch (err) {
      setError('Failed to connect to WebSocket');
    }
  }, [realtime, maxEvents]);
  
  // Initial load and WebSocket connection
  useEffect(() => {
    refresh();
    connectWebSocket();
    
    return () => {
      // Cleanup
      if (wsRef.current) {
        wsRef.current.close();
      }
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
    };
  }, [refresh, connectWebSocket]);
  
  return {
    events,
    loading,
    error,
    connected,
    refresh,
    clear,
  };
}

/**
 * Fetch events once (no real-time updates)
 */
export async function fetchEvents(): Promise<WebEvent[]> {
  const response = await fetch(`${API_BASE}/api/web-events`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  const data: WebEventsResponse = await response.json();
  return data.events;
}

