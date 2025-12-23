'use client';

import { useState, useMemo } from 'react';
import { WebEvent, WebEventType } from '@/types/event';
import { formatTimestamp, getEventTypeColor, parseEvent } from '@/utils/eventParsers';
import { 
  MagnifyingGlassIcon,
  FunnelIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  XMarkIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

interface LogViewProps {
  events: WebEvent[];
}

type FilterType = 'all' | WebEventType;

export function LogView({ events }: LogViewProps) {
  const [filter, setFilter] = useState<FilterType>('all');
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  
  // Filter events
  const filteredEvents = useMemo(() => {
    let result = events;
    
    // Type filter
    if (filter !== 'all') {
      result = result.filter(e => e.type === filter);
    }
    
    // Search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(e => 
        e.comm.toLowerCase().includes(query) ||
        e.type.toLowerCase().includes(query) ||
        JSON.stringify(e.data).toLowerCase().includes(query)
      );
    }
    
    return result;
  }, [events, filter, searchQuery]);
  
  // Get unique types for filter options
  const eventTypes = useMemo(() => {
    const types = new Set(events.map(e => e.type));
    return Array.from(types);
  }, [events]);
  
  const toggleEvent = (id: string) => {
    setExpandedEvents(prev => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const clearFilters = () => {
    setFilter('all');
    setSearchQuery('');
  };

  const hasActiveFilters = filter !== 'all' || searchQuery.trim().length > 0;
  
  return (
    <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold text-text-primary">
            Event Log
          </h2>
          <p className="text-xs text-text-muted mt-0.5">
            {filteredEvents.length} of {events.length} events
          </p>
        </div>
        
        {hasActiveFilters && (
          <button
            onClick={clearFilters}
            className="btn btn-ghost text-xs"
          >
            <XMarkIcon className="w-4 h-4" />
            Clear Filters
          </button>
        )}
      </div>
      
      {/* Filters */}
      <div className="px-6 py-3 border-b border-border flex items-center gap-4 bg-bg-tertiary/30">
        {/* Search */}
        <div className="flex-1 relative">
          <MagnifyingGlassIcon className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" />
          <input
            type="text"
            placeholder="Search events..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-9 pr-3 py-2 bg-bg-secondary"
          />
        </div>
        
        {/* Type Filter */}
        <div className="flex items-center gap-2">
          <FunnelIcon className="w-4 h-4 text-text-muted" />
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value as FilterType)}
            className="bg-bg-secondary min-w-[140px]"
          >
            <option value="all">All Types</option>
            {eventTypes.map(type => (
              <option key={type} value={type} className="capitalize">
                {type.replace('_', ' ')}
              </option>
            ))}
          </select>
        </div>
      </div>
      
      {/* Event List */}
      <div className="divide-y divide-border max-h-[600px] overflow-y-auto">
        {filteredEvents.length === 0 ? (
          <div className="px-6 py-16 text-center">
            <MagnifyingGlassIcon className="w-10 h-10 text-text-muted mx-auto mb-3" />
            <p className="text-text-secondary">No events match your filters</p>
            <p className="text-sm text-text-muted mt-1">
              Try adjusting your search or filter criteria
            </p>
          </div>
        ) : (
          filteredEvents.map(event => {
            const parsed = parseEvent(event);
            const isExpanded = expandedEvents.has(event.id);
            const color = getEventTypeColor(event.type);
            
            return (
              <div key={event.id} className="group">
                {/* Event Row */}
                <div
                  onClick={() => toggleEvent(event.id)}
                  className="px-6 py-3 flex items-center gap-4 cursor-pointer hover:bg-bg-tertiary/50 transition-colors"
                >
                  {/* Expand Icon */}
                  {isExpanded ? (
                    <ChevronDownIcon className="w-4 h-4 text-text-muted flex-shrink-0" />
                  ) : (
                    <ChevronRightIcon className="w-4 h-4 text-text-muted flex-shrink-0" />
                  )}
                  
                  {/* Timestamp */}
                  <span className="text-[10px] font-mono text-text-muted w-16 flex-shrink-0">
                    {formatTimestamp(event.timestamp)}
                  </span>
                  
                  {/* Event Type Badge */}
                  <span className={clsx(
                    'px-2 py-0.5 rounded text-[10px] font-medium flex-shrink-0 min-w-[80px] text-center',
                    color
                  )}>
                    {parsed.title}
                  </span>
                  
                  {/* Process */}
                  <span className="px-2 py-0.5 bg-bg-tertiary rounded text-[10px] font-mono text-text-muted flex-shrink-0">
                    {event.comm}:{event.pid}
                  </span>
                  
                  {/* Subtitle */}
                  <span className="text-sm text-text-secondary truncate flex-1 min-w-0">
                    {parsed.subtitle}
                  </span>
                </div>
                
                {/* Expanded Details */}
                {isExpanded && (
                  <div className="px-6 pb-4 pt-0 animate-slide-up">
                    <div className="ml-8 p-4 bg-bg-tertiary rounded-lg border border-border">
                      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                        <DetailItem label="Event ID" value={event.id} mono />
                        <DetailItem 
                          label="Timestamp" 
                          value={new Date(event.timestamp).toISOString()} 
                          mono 
                        />
                        <DetailItem 
                          label="Process" 
                          value={`${event.comm} (PID: ${event.pid}${event.ppid ? `, PPID: ${event.ppid}` : ''})`}
                        />
                        <DetailItem label="Type" value={event.type} />
                      </div>
                      
                      <div>
                        <span className="text-[10px] text-text-muted uppercase tracking-wider mb-2 block">
                          Event Data
                        </span>
                        <pre className="p-3 bg-bg-primary rounded-lg text-xs font-mono text-text-secondary overflow-x-auto border border-border">
                          {JSON.stringify(event.data, null, 2)}
                        </pre>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

function DetailItem({ 
  label, 
  value, 
  mono = false 
}: { 
  label: string; 
  value: string; 
  mono?: boolean;
}) {
  return (
    <div>
      <span className="text-[10px] text-text-muted uppercase tracking-wider block mb-1">
        {label}
      </span>
      <span className={clsx(
        'text-xs text-text-secondary break-all',
        mono && 'font-mono'
      )}>
        {value}
      </span>
    </div>
  );
}
