'use client';

import { ViewType } from '@/app/page';
import { Stats } from '@/lib/useStats';
import { ArrowPathIcon, SignalIcon } from '@heroicons/react/24/outline';
import clsx from 'clsx';

interface TopBarProps {
  connected: boolean;
  stats: Stats | null;
  onRefresh: () => void;
  currentView: ViewType;
}

const viewTitles: Record<ViewType, { title: string; description: string }> = {
  dashboard: { title: 'Dashboard', description: 'Overview of your AI agent activity' },
  tree: { title: 'Process Tree', description: 'Hierarchical view of processes and their AI calls' },
  timeline: { title: 'Timeline', description: 'Chronological event stream' },
  log: { title: 'Event Log', description: 'Raw event stream with filtering' },
  inventory: { title: 'Inventory', description: 'AI providers and applications overview' },
  traces: { title: 'Traces', description: 'Active and completed agent traces' },
  metrics: { title: 'Resource Metrics', description: 'CPU and memory usage per process' },
  settings: { title: 'Settings', description: 'Configure sinks, redaction, and preferences' },
};

export function TopBar({ connected, stats, onRefresh, currentView }: TopBarProps) {
  const { title, description } = viewTitles[currentView];
  
  return (
    <header className="h-16 flex items-center justify-between px-6 bg-bg-secondary border-b border-border">
      {/* Page Title */}
      <div>
        <h1 className="text-lg font-semibold text-text-primary">{title}</h1>
        <p className="text-xs text-text-muted">{description}</p>
      </div>
      
      {/* Stats & Actions */}
      <div className="flex items-center gap-6">
        {/* Quick Stats */}
        {stats && (
          <div className="hidden md:flex items-center gap-6">
            <QuickStat 
              label="Events" 
              value={stats.total_events} 
            />
            <QuickStat 
              label="AI Calls" 
              value={stats.ai_events} 
              color="text-accent-green"
            />
            <QuickStat 
              label="Active Traces" 
              value={stats.active_traces}
              color="text-accent-purple" 
            />
          </div>
        )}
        
        {/* Connection Status */}
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-bg-tertiary">
          <SignalIcon className={clsx('w-4 h-4', connected ? 'text-accent-green' : 'text-accent-red')} />
          <span className="text-xs text-text-secondary">
            {connected ? 'Live' : 'Disconnected'}
          </span>
        </div>
        
        {/* Refresh Button */}
        <button
          onClick={onRefresh}
          className="p-2 rounded-lg hover:bg-bg-tertiary transition-colors text-text-secondary hover:text-text-primary"
          title="Refresh data"
        >
          <ArrowPathIcon className="w-5 h-5" />
        </button>
      </div>
    </header>
  );
}

function QuickStat({ 
  label, 
  value, 
  color = 'text-text-primary' 
}: { 
  label: string; 
  value: number; 
  color?: string;
}) {
  return (
    <div className="text-right">
      <div className={clsx('text-lg font-mono font-semibold', color)}>
        {value.toLocaleString()}
      </div>
      <div className="text-[10px] text-text-muted uppercase tracking-wider">
        {label}
      </div>
    </div>
  );
}

