'use client';

import { ArrowPathIcon } from '@heroicons/react/24/outline';

interface HeaderProps {
  stats: {
    total: number;
    aiPrompts: number;
    aiResponses: number;
    processes: number;
  };
  connected: boolean;
  onRefresh: () => void;
}

export function Header({ stats, connected, onRefresh }: HeaderProps) {
  return (
    <header className="border-b border-border bg-bg-secondary/80 backdrop-blur-sm sticky top-0 z-50">
      <div className="max-w-[1800px] mx-auto px-4 py-4">
        <div className="flex items-center justify-between">
          {/* Logo */}
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-accent-blue to-accent-purple flex items-center justify-center">
              <span className="text-white font-bold text-sm">O</span>
            </div>
            <div>
              <h1 className="text-lg font-semibold text-text-primary">
                OISP Sensor
              </h1>
              <p className="text-xs text-text-muted">
                AI Agent Observability
              </p>
            </div>
          </div>
          
          {/* Stats */}
          <div className="hidden md:flex items-center gap-8">
            <Stat label="Events" value={stats.total} />
            <Stat label="AI Prompts" value={stats.aiPrompts} color="text-accent-green" />
            <Stat label="AI Responses" value={stats.aiResponses} color="text-accent-blue" />
            <Stat label="Processes" value={stats.processes} color="text-accent-purple" />
          </div>
          
          {/* Actions */}
          <div className="flex items-center gap-4">
            {/* Connection Status */}
            <div className="flex items-center gap-2">
              <div 
                className={`w-2 h-2 rounded-full ${
                  connected 
                    ? 'bg-accent-green live-indicator' 
                    : 'bg-accent-red'
                }`} 
              />
              <span className="text-xs text-text-secondary">
                {connected ? 'Live' : 'Disconnected'}
              </span>
            </div>
            
            {/* Refresh Button */}
            <button
              onClick={onRefresh}
              className="p-2 rounded-lg hover:bg-bg-tertiary transition-colors text-text-secondary hover:text-text-primary"
              title="Refresh events"
            >
              <ArrowPathIcon className="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>
    </header>
  );
}

function Stat({ 
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
      <div className={`text-xl font-mono font-semibold ${color}`}>
        {value.toLocaleString()}
      </div>
      <div className="text-xs text-text-muted uppercase tracking-wide">
        {label}
      </div>
    </div>
  );
}

