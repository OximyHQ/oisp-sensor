'use client';

import { useState, useEffect, useCallback } from 'react';

const API_BASE = typeof window !== 'undefined' 
  ? `${window.location.protocol}//${window.location.host}`
  : 'http://localhost:7777';

export interface ProviderInfo {
  name: string;
  request_count: number;
  models: string[];
}

export interface AppInfo {
  name: string;
  exe: string;
  request_count: number;
  providers: string[];
  account_type: string;
}

export interface Inventory {
  providers: ProviderInfo[];
  apps: AppInfo[];
}

interface UseInventoryReturn {
  inventory: Inventory | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useInventory(): UseInventoryReturn {
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await fetch(`${API_BASE}/api/inventory`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
      const data: Inventory = await response.json();
      setInventory(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch inventory');
    } finally {
      setLoading(false);
    }
  }, []);
  
  useEffect(() => {
    refresh();
    
    // Refresh every 10 seconds
    const interval = setInterval(refresh, 10000);
    return () => clearInterval(interval);
  }, [refresh]);
  
  return { inventory, loading, error, refresh };
}

