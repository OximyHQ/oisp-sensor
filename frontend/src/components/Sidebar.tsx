'use client';

import { ViewType } from '@/app/page';
import clsx from 'clsx';
import {
  HomeIcon,
  CpuChipIcon,
  ChartBarIcon,
  QueueListIcon,
  CubeIcon,
  ArrowPathIcon,
  Cog6ToothIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  SignalIcon,
  ChartPieIcon,
} from '@heroicons/react/24/outline';

interface SidebarProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  stats: {
    total: number;
    aiPrompts: number;
    aiResponses: number;
    processes: number;
  };
  connected: boolean;
}

const navItems: { id: ViewType; label: string; icon: typeof HomeIcon; badge?: 'ai' | 'process' }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: HomeIcon },
  { id: 'tree', label: 'Process Tree', icon: CpuChipIcon, badge: 'process' },
  { id: 'timeline', label: 'Timeline', icon: ChartBarIcon },
  { id: 'log', label: 'Event Log', icon: QueueListIcon },
  { id: 'inventory', label: 'Inventory', icon: CubeIcon },
  { id: 'traces', label: 'Traces', icon: ArrowPathIcon, badge: 'ai' },
  { id: 'metrics', label: 'Metrics', icon: ChartPieIcon },
];

export function Sidebar({
  currentView,
  onViewChange,
  collapsed,
  onToggleCollapse,
  stats,
  connected,
}: SidebarProps) {
  return (
    <aside
      className={clsx(
        'flex flex-col bg-bg-secondary border-r border-border transition-all duration-200',
        collapsed ? 'w-16' : 'w-56'
      )}
    >
      {/* Logo */}
      <div className="h-16 flex items-center justify-between px-4 border-b border-border">
        <div className="flex items-center gap-3 min-w-0">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-accent-blue to-accent-purple flex items-center justify-center flex-shrink-0">
            <span className="text-white font-bold text-sm">O</span>
          </div>
          {!collapsed && (
            <div className="min-w-0">
              <h1 className="text-sm font-semibold text-text-primary truncate">
                OISP Sensor
              </h1>
              <div className="flex items-center gap-1.5">
                <div 
                  className={clsx(
                    'w-1.5 h-1.5 rounded-full',
                    connected ? 'bg-accent-green live-indicator' : 'bg-accent-red'
                  )}
                />
                <span className="text-[10px] text-text-muted">
                  {connected ? 'Connected' : 'Offline'}
                </span>
              </div>
            </div>
          )}
        </div>
      </div>
      
      {/* Navigation */}
      <nav className="flex-1 py-4 px-2 space-y-1 overflow-y-auto">
        {navItems.map(({ id, label, icon: Icon, badge }) => {
          const isActive = currentView === id;
          const badgeCount = badge === 'ai' 
            ? stats.aiPrompts + stats.aiResponses 
            : badge === 'process' 
              ? stats.processes 
              : 0;
          
          return (
            <button
              key={id}
              onClick={() => onViewChange(id)}
              className={clsx(
                'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all',
                isActive
                  ? 'bg-accent-blue/10 text-accent-blue'
                  : 'text-text-secondary hover:text-text-primary hover:bg-bg-tertiary'
              )}
              title={collapsed ? label : undefined}
            >
              <Icon className={clsx('w-5 h-5 flex-shrink-0', isActive && 'text-accent-blue')} />
              {!collapsed && (
                <>
                  <span className="flex-1 text-left truncate">{label}</span>
                  {badge && badgeCount > 0 && (
                    <span 
                      className={clsx(
                        'px-1.5 py-0.5 rounded text-[10px] font-medium',
                        badge === 'ai' 
                          ? 'bg-accent-green/20 text-accent-green'
                          : 'bg-accent-purple/20 text-accent-purple'
                      )}
                    >
                      {badgeCount > 99 ? '99+' : badgeCount}
                    </span>
                  )}
                </>
              )}
            </button>
          );
        })}
      </nav>
      
      {/* Bottom Section */}
      <div className="p-2 border-t border-border space-y-1">
        {/* Settings */}
        <button
          onClick={() => onViewChange('settings')}
          className={clsx(
            'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all',
            currentView === 'settings'
              ? 'bg-accent-blue/10 text-accent-blue'
              : 'text-text-secondary hover:text-text-primary hover:bg-bg-tertiary'
          )}
          title={collapsed ? 'Settings' : undefined}
        >
          <Cog6ToothIcon className="w-5 h-5 flex-shrink-0" />
          {!collapsed && <span className="flex-1 text-left">Settings</span>}
        </button>
        
        {/* Collapse Toggle */}
        <button
          onClick={onToggleCollapse}
          className="w-full flex items-center justify-center gap-3 px-3 py-2 rounded-lg text-text-muted hover:text-text-secondary hover:bg-bg-tertiary transition-all"
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {collapsed ? (
            <ChevronRightIcon className="w-4 h-4" />
          ) : (
            <>
              <ChevronLeftIcon className="w-4 h-4" />
              <span className="text-xs">Collapse</span>
            </>
          )}
        </button>
      </div>
    </aside>
  );
}

