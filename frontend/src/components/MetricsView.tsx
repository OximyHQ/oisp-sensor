'use client';

import { useMemo } from 'react';
import { useProcessMetrics, ProcessResourceInfo } from '@/lib/useProcessMetrics';
import clsx from 'clsx';
import {
  ArrowPathIcon,
  CpuChipIcon,
  CircleStackIcon,
  ChartBarIcon,
  ServerIcon,
} from '@heroicons/react/24/outline';

export function MetricsView() {
  const { metrics, loading, error, refresh } = useProcessMetrics();

  // Sort by CPU usage descending
  const sortedByCpu = useMemo(() => {
    return [...metrics].sort((a, b) => b.cpu_percent - a.cpu_percent);
  }, [metrics]);

  // Sort by memory usage descending
  const sortedByMemory = useMemo(() => {
    return [...metrics].sort((a, b) => b.memory_rss_mb - a.memory_rss_mb);
  }, [metrics]);

  // Calculate totals
  const totals = useMemo(() => {
    const totalCpu = metrics.reduce((acc, m) => acc + m.cpu_percent, 0);
    const totalRss = metrics.reduce((acc, m) => acc + m.memory_rss_mb, 0);
    const totalVms = metrics.reduce((acc, m) => acc + m.memory_vms_mb, 0);
    return { cpu: totalCpu, rss: totalRss, vms: totalVms, count: metrics.length };
  }, [metrics]);

  if (loading && metrics.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="w-8 h-8 border-2 border-accent-blue border-t-transparent rounded-full animate-spin mx-auto mb-4" />
          <p className="text-text-secondary">Loading process metrics...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-text-primary">Resource Metrics</h2>
          <p className="text-sm text-text-muted mt-1">
            Real-time CPU and memory usage per process
          </p>
        </div>
        <button
          onClick={refresh}
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

      {error && (
        <div className="p-4 bg-accent-red/10 border border-accent-red/30 rounded-lg text-accent-red text-sm">
          {error}
        </div>
      )}

      {/* Summary Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <SummaryCard
          title="Tracked Processes"
          value={totals.count}
          icon={ServerIcon}
          color="purple"
        />
        <SummaryCard
          title="Total CPU"
          value={`${totals.cpu.toFixed(1)}%`}
          icon={CpuChipIcon}
          color="blue"
        />
        <SummaryCard
          title="Total RSS Memory"
          value={formatMemory(totals.rss)}
          icon={CircleStackIcon}
          color="green"
        />
        <SummaryCard
          title="Total Virtual Memory"
          value={formatMemory(totals.vms)}
          icon={ChartBarIcon}
          color="cyan"
        />
      </div>

      {metrics.length === 0 ? (
        <div className="bg-bg-secondary rounded-xl border border-border p-12 text-center">
          <CpuChipIcon className="w-12 h-12 text-text-muted mx-auto mb-4" />
          <p className="text-text-secondary">No process metrics available</p>
          <p className="text-sm text-text-muted mt-1">
            Process metrics will appear here when the sensor is capturing events
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* CPU Usage Table */}
          <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
            <div className="px-6 py-4 border-b border-border flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-accent-blue/10 flex items-center justify-center">
                <CpuChipIcon className="w-5 h-5 text-accent-blue" />
              </div>
              <div>
                <h3 className="text-sm font-semibold text-text-primary">CPU Usage</h3>
                <p className="text-xs text-text-muted">Sorted by usage (highest first)</p>
              </div>
            </div>

            <div className="divide-y divide-border max-h-[400px] overflow-y-auto">
              {sortedByCpu.slice(0, 15).map((process) => (
                <ProcessCpuRow key={process.pid} process={process} maxCpu={sortedByCpu[0]?.cpu_percent || 100} />
              ))}
            </div>
          </div>

          {/* Memory Usage Table */}
          <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
            <div className="px-6 py-4 border-b border-border flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-accent-green/10 flex items-center justify-center">
                <CircleStackIcon className="w-5 h-5 text-accent-green" />
              </div>
              <div>
                <h3 className="text-sm font-semibold text-text-primary">Memory Usage</h3>
                <p className="text-xs text-text-muted">RSS memory (highest first)</p>
              </div>
            </div>

            <div className="divide-y divide-border max-h-[400px] overflow-y-auto">
              {sortedByMemory.slice(0, 15).map((process) => (
                <ProcessMemoryRow key={process.pid} process={process} maxRss={sortedByMemory[0]?.memory_rss_mb || 100} />
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Full Process Table */}
      {metrics.length > 0 && (
        <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
          <div className="px-6 py-4 border-b border-border">
            <h3 className="text-sm font-semibold text-text-primary">All Tracked Processes</h3>
            <p className="text-xs text-text-muted mt-0.5">
              Complete list of processes with resource usage
            </p>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="bg-bg-tertiary/50">
                  <th className="table-header">Process</th>
                  <th className="table-header">PID</th>
                  <th className="table-header text-right">CPU %</th>
                  <th className="table-header text-right">RSS</th>
                  <th className="table-header text-right">Virtual</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {sortedByCpu.map((process) => (
                  <tr key={process.pid} className="hover:bg-bg-tertiary/30 transition-colors">
                    <td className="table-cell">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-lg bg-accent-purple/10 flex items-center justify-center">
                          <CpuChipIcon className="w-4 h-4 text-accent-purple" />
                        </div>
                        <span className="font-medium text-text-primary">{process.comm}</span>
                      </div>
                    </td>
                    <td className="table-cell">
                      <span className="font-mono text-text-muted">{process.pid}</span>
                    </td>
                    <td className="table-cell text-right">
                      <span className={clsx(
                        'font-mono font-medium',
                        process.cpu_percent > 50 ? 'text-accent-red' :
                        process.cpu_percent > 20 ? 'text-accent-orange' :
                        'text-text-primary'
                      )}>
                        {process.cpu_percent.toFixed(1)}%
                      </span>
                    </td>
                    <td className="table-cell text-right">
                      <span className="font-mono text-text-secondary">
                        {formatMemory(process.memory_rss_mb)}
                      </span>
                    </td>
                    <td className="table-cell text-right">
                      <span className="font-mono text-text-muted">
                        {formatMemory(process.memory_vms_mb)}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

function SummaryCard({
  title,
  value,
  icon: Icon,
  color,
}: {
  title: string;
  value: string | number;
  icon: typeof CpuChipIcon;
  color: 'blue' | 'green' | 'purple' | 'cyan';
}) {
  const colorClasses = {
    blue: 'text-accent-blue bg-accent-blue/10',
    green: 'text-accent-green bg-accent-green/10',
    purple: 'text-accent-purple bg-accent-purple/10',
    cyan: 'text-accent-cyan bg-accent-cyan/10',
  };

  return (
    <div className="bg-bg-secondary rounded-xl border border-border p-5">
      <div className="flex items-center gap-3 mb-3">
        <div className={clsx('w-10 h-10 rounded-xl flex items-center justify-center', colorClasses[color])}>
          <Icon className="w-5 h-5" />
        </div>
      </div>
      <p className="text-2xl font-mono font-bold text-text-primary">{value}</p>
      <p className="text-xs text-text-muted mt-1">{title}</p>
    </div>
  );
}

function ProcessCpuRow({ process, maxCpu }: { process: ProcessResourceInfo; maxCpu: number }) {
  const barWidth = maxCpu > 0 ? (process.cpu_percent / maxCpu) * 100 : 0;
  
  return (
    <div className="px-6 py-3 hover:bg-bg-tertiary/30 transition-colors">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-medium text-sm text-text-primary truncate">{process.comm}</span>
          <span className="text-[10px] font-mono text-text-muted">PID {process.pid}</span>
        </div>
        <span className={clsx(
          'text-sm font-mono font-medium',
          process.cpu_percent > 50 ? 'text-accent-red' :
          process.cpu_percent > 20 ? 'text-accent-orange' :
          'text-accent-blue'
        )}>
          {process.cpu_percent.toFixed(1)}%
        </span>
      </div>
      <div className="h-1.5 bg-bg-tertiary rounded-full overflow-hidden">
        <div
          className={clsx(
            'h-full rounded-full transition-all duration-500',
            process.cpu_percent > 50 ? 'bg-accent-red' :
            process.cpu_percent > 20 ? 'bg-accent-orange' :
            'bg-accent-blue'
          )}
          style={{ width: `${barWidth}%` }}
        />
      </div>
    </div>
  );
}

function ProcessMemoryRow({ process, maxRss }: { process: ProcessResourceInfo; maxRss: number }) {
  const barWidth = maxRss > 0 ? (process.memory_rss_mb / maxRss) * 100 : 0;
  
  return (
    <div className="px-6 py-3 hover:bg-bg-tertiary/30 transition-colors">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-medium text-sm text-text-primary truncate">{process.comm}</span>
          <span className="text-[10px] font-mono text-text-muted">PID {process.pid}</span>
        </div>
        <span className="text-sm font-mono font-medium text-accent-green">
          {formatMemory(process.memory_rss_mb)}
        </span>
      </div>
      <div className="h-1.5 bg-bg-tertiary rounded-full overflow-hidden">
        <div
          className="h-full rounded-full bg-accent-green transition-all duration-500"
          style={{ width: `${barWidth}%` }}
        />
      </div>
    </div>
  );
}

function formatMemory(mb: number): string {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${mb.toFixed(1)} MB`;
}

