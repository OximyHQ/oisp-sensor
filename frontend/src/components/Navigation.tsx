'use client';

import { 
  QueueListIcon, 
  ChartBarIcon, 
  CircleStackIcon 
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

type ViewType = 'tree' | 'timeline' | 'log';

interface NavigationProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
  eventCount: number;
}

const views: { id: ViewType; label: string; icon: typeof QueueListIcon }[] = [
  { id: 'tree', label: 'Process Tree', icon: CircleStackIcon },
  { id: 'timeline', label: 'Timeline', icon: ChartBarIcon },
  { id: 'log', label: 'Log View', icon: QueueListIcon },
];

export function Navigation({ currentView, onViewChange, eventCount }: NavigationProps) {
  return (
    <nav className="border-b border-border bg-bg-secondary/50">
      <div className="max-w-[1800px] mx-auto px-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1">
            {views.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                onClick={() => onViewChange(id)}
                className={clsx(
                  'flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors relative',
                  currentView === id
                    ? 'text-accent-blue'
                    : 'text-text-secondary hover:text-text-primary'
                )}
              >
                <Icon className="w-4 h-4" />
                {label}
                
                {/* Active indicator */}
                {currentView === id && (
                  <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-blue" />
                )}
              </button>
            ))}
          </div>
          
          <div className="text-sm text-text-muted">
            {eventCount.toLocaleString()} events loaded
          </div>
        </div>
      </div>
    </nav>
  );
}

