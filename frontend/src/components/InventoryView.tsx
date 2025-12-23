'use client';

import { Inventory } from '@/lib/useInventory';
import clsx from 'clsx';
import {
  ArrowPathIcon,
  CubeIcon,
  CpuChipIcon,
  CloudIcon,
  ServerIcon,
} from '@heroicons/react/24/outline';

interface InventoryViewProps {
  inventory: Inventory | null;
  loading: boolean;
  onRefresh: () => void;
}

export function InventoryView({ inventory, loading, onRefresh }: InventoryViewProps) {
  if (loading && !inventory) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="w-8 h-8 border-2 border-accent-blue border-t-transparent rounded-full animate-spin mx-auto mb-4" />
          <p className="text-text-secondary">Loading inventory...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-text-primary">AI Providers & Applications</h2>
          <p className="text-sm text-text-muted mt-1">
            Overview of all AI providers and applications making LLM calls
          </p>
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

      {/* Providers Section */}
      <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
        <div className="px-6 py-4 border-b border-border flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-accent-green/10 flex items-center justify-center">
            <CloudIcon className="w-5 h-5 text-accent-green" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-text-primary">AI Providers</h3>
            <p className="text-xs text-text-muted">
              {inventory?.providers.length || 0} provider{inventory?.providers.length !== 1 ? 's' : ''} detected
            </p>
          </div>
        </div>

        {(!inventory || inventory.providers.length === 0) ? (
          <div className="px-6 py-12 text-center">
            <CubeIcon className="w-12 h-12 text-text-muted mx-auto mb-4" />
            <p className="text-text-secondary">No AI providers detected yet</p>
            <p className="text-sm text-text-muted mt-1">
              Providers will appear here once your agents make LLM calls
            </p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {inventory.providers.map((provider) => (
              <div key={provider.name} className="px-6 py-4 hover:bg-bg-tertiary/50 transition-colors">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-accent-green/20 to-accent-cyan/20 flex items-center justify-center">
                      <span className="text-accent-green font-bold text-lg">
                        {provider.name.charAt(0).toUpperCase()}
                      </span>
                    </div>
                    <div>
                      <h4 className="text-base font-semibold text-text-primary capitalize">
                        {provider.name}
                      </h4>
                      <p className="text-sm text-text-muted mt-0.5">
                        {provider.models.length} model{provider.models.length !== 1 ? 's' : ''} used
                      </p>
                    </div>
                  </div>
                  
                  <div className="text-right">
                    <p className="text-2xl font-mono font-bold text-accent-green">
                      {provider.request_count.toLocaleString()}
                    </p>
                    <p className="text-xs text-text-muted">requests</p>
                  </div>
                </div>
                
                {/* Models */}
                <div className="mt-4 flex flex-wrap gap-2">
                  {provider.models.map((model) => (
                    <span
                      key={model}
                      className="px-3 py-1.5 bg-bg-tertiary rounded-lg text-xs font-mono text-text-secondary"
                    >
                      {model}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Applications Section */}
      <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
        <div className="px-6 py-4 border-b border-border flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-accent-purple/10 flex items-center justify-center">
            <CpuChipIcon className="w-5 h-5 text-accent-purple" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-text-primary">Applications</h3>
            <p className="text-xs text-text-muted">
              {inventory?.apps.length || 0} application{inventory?.apps.length !== 1 ? 's' : ''} tracked
            </p>
          </div>
        </div>

        {(!inventory || inventory.apps.length === 0) ? (
          <div className="px-6 py-12 text-center">
            <ServerIcon className="w-12 h-12 text-text-muted mx-auto mb-4" />
            <p className="text-text-secondary">No applications tracked yet</p>
            <p className="text-sm text-text-muted mt-1">
              Applications will appear here as they make AI API calls
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="bg-bg-tertiary/50">
                  <th className="table-header">Application</th>
                  <th className="table-header">Executable</th>
                  <th className="table-header">Providers</th>
                  <th className="table-header text-right">Requests</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {inventory.apps.map((app) => (
                  <tr key={app.name} className="hover:bg-bg-tertiary/30 transition-colors">
                    <td className="table-cell">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-lg bg-accent-purple/10 flex items-center justify-center">
                          <CpuChipIcon className="w-4 h-4 text-accent-purple" />
                        </div>
                        <span className="font-medium text-text-primary">{app.name}</span>
                      </div>
                    </td>
                    <td className="table-cell">
                      <span className="font-mono text-xs text-text-muted truncate block max-w-xs">
                        {app.exe || '--'}
                      </span>
                    </td>
                    <td className="table-cell">
                      <div className="flex flex-wrap gap-1">
                        {app.providers.map((provider) => (
                          <span
                            key={provider}
                            className="px-2 py-0.5 bg-accent-green/10 text-accent-green rounded text-[10px] font-medium capitalize"
                          >
                            {provider}
                          </span>
                        ))}
                      </div>
                    </td>
                    <td className="table-cell text-right">
                      <span className="font-mono font-semibold text-text-primary">
                        {app.request_count.toLocaleString()}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

