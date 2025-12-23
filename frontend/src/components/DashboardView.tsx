'use client';

import { useMemo } from 'react';
import { WebEvent } from '@/types/event';
import { Stats } from '@/lib/useStats';
import { Inventory } from '@/lib/useInventory';
import { TracesResponse } from '@/lib/useTraces';
import { ViewType } from '@/app/page';
import { formatTimestamp, getEventTypeColor, parseEvent } from '@/utils/eventParsers';
import clsx from 'clsx';
import {
  BoltIcon,
  CpuChipIcon,
  ChatBubbleLeftRightIcon,
  ArrowPathIcon,
  ClockIcon,
  ChevronRightIcon,
  CubeIcon,
  SignalIcon,
} from '@heroicons/react/24/outline';

interface DashboardViewProps {
  events: WebEvent[];
  stats: Stats | null;
  inventory: Inventory | null;
  traces: TracesResponse | null;
  connected: boolean;
  onNavigate: (view: ViewType) => void;
}

export function DashboardView({
  events,
  stats,
  inventory,
  traces,
  connected,
  onNavigate,
}: DashboardViewProps) {
  // Calculate additional stats
  const dashboardStats = useMemo(() => {
    const aiPrompts = events.filter(e => e.type === 'ai_prompt').length;
    const aiResponses = events.filter(e => e.type === 'ai_response').length;
    const uniqueProcesses = new Set(events.map(e => e.pid)).size;
    const uniqueProviders = new Set(
      events
        .filter(e => e.type === 'ai_prompt' || e.type === 'ai_response')
        .map(e => (e.data as { provider?: string }).provider)
        .filter(Boolean)
    ).size;
    
    // Calculate tokens from responses
    let totalTokens = 0;
    for (const event of events) {
      if (event.type === 'ai_response') {
        const data = event.data as { input_tokens?: number; output_tokens?: number };
        totalTokens += (data.input_tokens || 0) + (data.output_tokens || 0);
      }
    }
    
    return { aiPrompts, aiResponses, uniqueProcesses, uniqueProviders, totalTokens };
  }, [events]);

  // Get recent events for the activity feed
  const recentEvents = useMemo(() => {
    return events.slice(0, 8);
  }, [events]);

  return (
    <div className="space-y-6 stagger-children">
      {/* Stats Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4">
        <StatCard
          title="Total Events"
          value={stats?.total_events ?? events.length}
          icon={BoltIcon}
          color="blue"
        />
        <StatCard
          title="AI Calls"
          value={dashboardStats.aiPrompts + dashboardStats.aiResponses}
          icon={ChatBubbleLeftRightIcon}
          color="green"
          subtitle={`${dashboardStats.aiPrompts} req / ${dashboardStats.aiResponses} res`}
        />
        <StatCard
          title="Active Traces"
          value={traces?.active ?? 0}
          icon={ArrowPathIcon}
          color="purple"
          subtitle={`${traces?.completed ?? 0} completed`}
        />
        <StatCard
          title="Processes"
          value={dashboardStats.uniqueProcesses}
          icon={CpuChipIcon}
          color="cyan"
        />
        <StatCard
          title="Total Tokens"
          value={dashboardStats.totalTokens}
          icon={CubeIcon}
          color="orange"
          format="compact"
        />
      </div>
      
      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Activity Feed */}
        <div className="lg:col-span-2 bg-bg-secondary rounded-xl border border-border overflow-hidden">
          <div className="px-5 py-4 border-b border-border flex items-center justify-between">
            <div>
              <h2 className="text-sm font-semibold text-text-primary">Recent Activity</h2>
              <p className="text-xs text-text-muted mt-0.5">Latest events from your agents</p>
            </div>
            <button
              onClick={() => onNavigate('log')}
              className="flex items-center gap-1 text-xs text-accent-blue hover:text-blue-400 transition-colors"
            >
              View all <ChevronRightIcon className="w-3 h-3" />
            </button>
          </div>
          
          <div className="divide-y divide-border">
            {recentEvents.length === 0 ? (
              <div className="px-5 py-12 text-center">
                <SignalIcon className="w-8 h-8 text-text-muted mx-auto mb-3" />
                <p className="text-sm text-text-secondary">No events yet</p>
                <p className="text-xs text-text-muted mt-1">
                  Start your AI agent to see activity here
                </p>
              </div>
            ) : (
              recentEvents.map((event) => {
                const parsed = parseEvent(event);
                const color = getEventTypeColor(event.type);
                
                return (
                  <div
                    key={event.id}
                    className="px-5 py-3 flex items-center gap-4 hover:bg-bg-tertiary/50 transition-colors"
                  >
                    <div
                      className={clsx(
                        'w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0',
                        color
                      )}
                    >
                      <EventIcon type={event.type} />
                    </div>
                    
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-text-primary">
                          {parsed.title}
                        </span>
                        <span className="text-xs text-text-muted truncate">
                          {parsed.subtitle}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 mt-0.5">
                        <span className="text-[10px] text-text-muted font-mono">
                          {event.comm}
                        </span>
                        <span className="text-[10px] text-text-muted">
                          PID {event.pid}
                        </span>
                      </div>
                    </div>
                    
                    <span className="text-[10px] text-text-muted font-mono flex-shrink-0">
                      {formatTimestamp(event.timestamp)}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        </div>
        
        {/* Right Column */}
        <div className="space-y-6">
          {/* Providers */}
          <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
            <div className="px-5 py-4 border-b border-border flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-text-primary">AI Providers</h2>
                <p className="text-xs text-text-muted mt-0.5">Active integrations</p>
              </div>
              <button
                onClick={() => onNavigate('inventory')}
                className="flex items-center gap-1 text-xs text-accent-blue hover:text-blue-400 transition-colors"
              >
                View all <ChevronRightIcon className="w-3 h-3" />
              </button>
            </div>
            
            <div className="p-4 space-y-3">
              {(!inventory || inventory.providers.length === 0) ? (
                <div className="text-center py-6">
                  <CubeIcon className="w-6 h-6 text-text-muted mx-auto mb-2" />
                  <p className="text-xs text-text-muted">No providers detected yet</p>
                </div>
              ) : (
                inventory.providers.slice(0, 4).map((provider) => (
                  <div
                    key={provider.name}
                    className="flex items-center justify-between p-3 bg-bg-tertiary rounded-lg"
                  >
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-accent-green/20 flex items-center justify-center">
                        <span className="text-accent-green font-bold text-xs">
                          {provider.name.charAt(0).toUpperCase()}
                        </span>
                      </div>
                      <div>
                        <p className="text-sm font-medium text-text-primary capitalize">
                          {provider.name}
                        </p>
                        <p className="text-[10px] text-text-muted">
                          {provider.models.length} model{provider.models.length !== 1 ? 's' : ''}
                        </p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="text-sm font-mono text-text-primary">
                        {provider.request_count}
                      </p>
                      <p className="text-[10px] text-text-muted">requests</p>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
          
          {/* Active Traces */}
          <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
            <div className="px-5 py-4 border-b border-border flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-text-primary">Active Traces</h2>
                <p className="text-xs text-text-muted mt-0.5">Running agent sessions</p>
              </div>
              <button
                onClick={() => onNavigate('traces')}
                className="flex items-center gap-1 text-xs text-accent-blue hover:text-blue-400 transition-colors"
              >
                View all <ChevronRightIcon className="w-3 h-3" />
              </button>
            </div>
            
            <div className="p-4 space-y-3">
              {(!traces || traces.traces.filter(t => !t.is_complete).length === 0) ? (
                <div className="text-center py-6">
                  <ArrowPathIcon className="w-6 h-6 text-text-muted mx-auto mb-2" />
                  <p className="text-xs text-text-muted">No active traces</p>
                </div>
              ) : (
                traces.traces
                  .filter(t => !t.is_complete)
                  .slice(0, 3)
                  .map((trace) => (
                    <div
                      key={trace.trace_id}
                      className="p-3 bg-bg-tertiary rounded-lg"
                    >
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-sm font-medium text-text-primary">
                          {trace.process_name || 'Unknown'}
                        </span>
                        <span className="flex items-center gap-1 text-[10px] text-accent-green">
                          <div className="w-1.5 h-1.5 rounded-full bg-accent-green live-indicator" />
                          Running
                        </span>
                      </div>
                      <div className="flex items-center gap-4 text-[10px] text-text-muted">
                        <span>{trace.llm_calls} LLM calls</span>
                        <span>{trace.tool_calls} tool calls</span>
                        <span>{trace.total_tokens.toLocaleString()} tokens</span>
                      </div>
                    </div>
                  ))
              )}
            </div>
          </div>
          
          {/* Connection Status */}
          <div className="bg-bg-secondary rounded-xl border border-border p-5">
            <div className="flex items-center gap-3 mb-4">
              <div
                className={clsx(
                  'w-10 h-10 rounded-xl flex items-center justify-center',
                  connected ? 'bg-accent-green/20' : 'bg-accent-red/20'
                )}
              >
                <SignalIcon
                  className={clsx(
                    'w-5 h-5',
                    connected ? 'text-accent-green' : 'text-accent-red'
                  )}
                />
              </div>
              <div>
                <p className="text-sm font-medium text-text-primary">
                  {connected ? 'Sensor Connected' : 'Sensor Disconnected'}
                </p>
                <p className="text-xs text-text-muted">
                  {connected ? 'Receiving real-time events' : 'Attempting to reconnect...'}
                </p>
              </div>
            </div>
            
            <div className="grid grid-cols-2 gap-3">
              <div className="p-3 bg-bg-tertiary rounded-lg">
                <p className="text-lg font-mono font-semibold text-text-primary">
                  {stats?.uptime_seconds ? formatUptime(stats.uptime_seconds) : '--'}
                </p>
                <p className="text-[10px] text-text-muted">Uptime</p>
              </div>
              <div className="p-3 bg-bg-tertiary rounded-lg">
                <p className="text-lg font-mono font-semibold text-text-primary">
                  {dashboardStats.uniqueProviders}
                </p>
                <p className="text-[10px] text-text-muted">Providers</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function StatCard({
  title,
  value,
  icon: Icon,
  color,
  subtitle,
  format = 'number',
}: {
  title: string;
  value: number;
  icon: typeof BoltIcon;
  color: 'blue' | 'green' | 'purple' | 'cyan' | 'orange' | 'red';
  subtitle?: string;
  format?: 'number' | 'compact';
}) {
  const colorClasses = {
    blue: 'text-accent-blue bg-accent-blue/10',
    green: 'text-accent-green bg-accent-green/10',
    purple: 'text-accent-purple bg-accent-purple/10',
    cyan: 'text-accent-cyan bg-accent-cyan/10',
    orange: 'text-accent-orange bg-accent-orange/10',
    red: 'text-accent-red bg-accent-red/10',
  };

  const formattedValue =
    format === 'compact' && value >= 1000
      ? value >= 1000000
        ? `${(value / 1000000).toFixed(1)}M`
        : `${(value / 1000).toFixed(1)}K`
      : value.toLocaleString();

  return (
    <div className="stat-card bg-bg-secondary rounded-xl border border-border p-5">
      <div className="flex items-center justify-between mb-3">
        <div className={clsx('w-10 h-10 rounded-xl flex items-center justify-center', colorClasses[color])}>
          <Icon className="w-5 h-5" />
        </div>
      </div>
      <div>
        <p className="text-2xl font-mono font-bold text-text-primary">{formattedValue}</p>
        <p className="text-xs text-text-muted mt-1">{title}</p>
        {subtitle && <p className="text-[10px] text-text-muted mt-0.5">{subtitle}</p>}
      </div>
    </div>
  );
}

function EventIcon({ type }: { type: string }) {
  const className = 'w-4 h-4';
  
  switch (type) {
    case 'ai_prompt':
      return (
        <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 11l5-5m0 0l5 5m-5-5v12" />
        </svg>
      );
    case 'ai_response':
      return (
        <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 13l-5 5m0 0l-5-5m5 5V6" />
        </svg>
      );
    case 'process_exec':
      return (
        <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      );
    case 'process_exit':
      return (
        <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
        </svg>
      );
    default:
      return <ClockIcon className={className} />;
  }
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

