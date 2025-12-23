'use client';

import { TracesResponse, TraceInfo } from '@/lib/useTraces';
import clsx from 'clsx';
import {
  ArrowPathIcon,
  ClockIcon,
  CpuChipIcon,
  ChatBubbleLeftRightIcon,
  WrenchScrewdriverIcon,
  CubeIcon,
  CheckCircleIcon,
  PlayCircleIcon,
} from '@heroicons/react/24/outline';

interface TracesViewProps {
  traces: TracesResponse | null;
  loading: boolean;
  onRefresh: () => void;
}

export function TracesView({ traces, loading, onRefresh }: TracesViewProps) {
  if (loading && !traces) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="w-8 h-8 border-2 border-accent-blue border-t-transparent rounded-full animate-spin mx-auto mb-4" />
          <p className="text-text-secondary">Loading traces...</p>
        </div>
      </div>
    );
  }

  const activeTraces = traces?.traces.filter(t => !t.is_complete) || [];
  const completedTraces = traces?.traces.filter(t => t.is_complete) || [];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-text-primary">Agent Traces</h2>
          <p className="text-sm text-text-muted mt-1">
            Track agent sessions, LLM calls, and tool usage
          </p>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-6 text-sm">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-accent-green live-indicator" />
              <span className="text-text-secondary">{traces?.active || 0} active</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-text-muted" />
              <span className="text-text-secondary">{traces?.completed || 0} completed</span>
            </div>
          </div>
          <button
            onClick={onRefresh}
            disabled={loading}
            className={clsx(
              'btn btn-secondary',
              loading && 'opacity-50 cursor-not-allowed'
            )}
          >
            <ArrowPathIcon className={clsx('w-4 h-4', loading && 'animate-spin')} />
            Refresh
          </button>
        </div>
      </div>

      {/* Active Traces */}
      <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
        <div className="px-6 py-4 border-b border-border flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-accent-green/10 flex items-center justify-center">
            <PlayCircleIcon className="w-5 h-5 text-accent-green" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-text-primary">Active Traces</h3>
            <p className="text-xs text-text-muted">Currently running agent sessions</p>
          </div>
        </div>

        {activeTraces.length === 0 ? (
          <div className="px-6 py-12 text-center">
            <PlayCircleIcon className="w-12 h-12 text-text-muted mx-auto mb-4" />
            <p className="text-text-secondary">No active traces</p>
            <p className="text-sm text-text-muted mt-1">
              Active agent sessions will appear here
            </p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {activeTraces.map((trace) => (
              <TraceCard key={trace.trace_id} trace={trace} />
            ))}
          </div>
        )}
      </div>

      {/* Completed Traces */}
      <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
        <div className="px-6 py-4 border-b border-border flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-text-muted/10 flex items-center justify-center">
            <CheckCircleIcon className="w-5 h-5 text-text-muted" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-text-primary">Completed Traces</h3>
            <p className="text-xs text-text-muted">Finished agent sessions</p>
          </div>
        </div>

        {completedTraces.length === 0 ? (
          <div className="px-6 py-12 text-center">
            <CheckCircleIcon className="w-12 h-12 text-text-muted mx-auto mb-4" />
            <p className="text-text-secondary">No completed traces yet</p>
            <p className="text-sm text-text-muted mt-1">
              Completed sessions will be shown here
            </p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {completedTraces.slice(0, 10).map((trace) => (
              <TraceCard key={trace.trace_id} trace={trace} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function TraceCard({ trace }: { trace: TraceInfo }) {
  const startedAt = new Date(trace.started_at);
  const isActive = !trace.is_complete;
  
  return (
    <div className="px-6 py-5 hover:bg-bg-tertiary/30 transition-colors">
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-4">
          <div className={clsx(
            'w-12 h-12 rounded-xl flex items-center justify-center',
            isActive ? 'bg-accent-green/10' : 'bg-bg-tertiary'
          )}>
            <CpuChipIcon className={clsx(
              'w-6 h-6',
              isActive ? 'text-accent-green' : 'text-text-muted'
            )} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="text-base font-semibold text-text-primary">
                {trace.process_name || 'Unknown Process'}
              </h4>
              {isActive && (
                <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-accent-green/10 text-accent-green text-[10px] font-medium">
                  <div className="w-1.5 h-1.5 rounded-full bg-accent-green live-indicator" />
                  Running
                </span>
              )}
            </div>
            <p className="text-xs text-text-muted mt-0.5 font-mono">
              {trace.trace_id.slice(0, 8)}...{trace.trace_id.slice(-8)}
            </p>
          </div>
        </div>
        
        <div className="text-right">
          <p className="text-sm text-text-secondary">
            {startedAt.toLocaleTimeString()}
          </p>
          <p className="text-xs text-text-muted">
            {startedAt.toLocaleDateString()}
          </p>
        </div>
      </div>
      
      {/* Stats Grid */}
      <div className="grid grid-cols-4 gap-4">
        <StatBlock
          icon={ClockIcon}
          label="Duration"
          value={formatDuration(trace.duration_ms)}
          color="cyan"
        />
        <StatBlock
          icon={ChatBubbleLeftRightIcon}
          label="LLM Calls"
          value={trace.llm_calls.toString()}
          color="green"
        />
        <StatBlock
          icon={WrenchScrewdriverIcon}
          label="Tool Calls"
          value={trace.tool_calls.toString()}
          color="purple"
        />
        <StatBlock
          icon={CubeIcon}
          label="Tokens"
          value={formatCompact(trace.total_tokens)}
          color="orange"
        />
      </div>
    </div>
  );
}

function StatBlock({
  icon: Icon,
  label,
  value,
  color,
}: {
  icon: typeof ClockIcon;
  label: string;
  value: string;
  color: 'green' | 'purple' | 'cyan' | 'orange';
}) {
  const colorClasses = {
    green: 'text-accent-green bg-accent-green/10',
    purple: 'text-accent-purple bg-accent-purple/10',
    cyan: 'text-accent-cyan bg-accent-cyan/10',
    orange: 'text-accent-orange bg-accent-orange/10',
  };

  return (
    <div className="p-3 bg-bg-tertiary rounded-lg">
      <div className="flex items-center gap-2 mb-1">
        <Icon className={clsx('w-4 h-4', colorClasses[color].split(' ')[0])} />
        <span className="text-[10px] text-text-muted uppercase tracking-wider">{label}</span>
      </div>
      <p className="text-lg font-mono font-semibold text-text-primary">{value}</p>
    </div>
  );
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  if (ms < 3600000) return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
  return `${Math.floor(ms / 3600000)}h ${Math.floor((ms % 3600000) / 60000)}m`;
}

function formatCompact(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return n.toString();
}

