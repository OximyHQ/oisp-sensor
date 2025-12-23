'use client';

import { useState, useMemo } from 'react';
import { WebEvent, WebEventType } from '@/types/event';
import { formatTimestamp, getEventTypeColor, parseEvent } from '@/utils/eventParsers';
import { 
  FunnelIcon,
  ChevronDownIcon,
  ChevronRightIcon,
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
  
  return (
    <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-text-primary">
            Log View
          </h2>
          <p className="text-sm text-text-muted mt-1">
            Raw event stream with filtering
          </p>
        </div>
        
        <div className="text-sm text-text-muted">
          {filteredEvents.length} of {events.length} events
        </div>
      </div>
      
      {/* Filters */}
      <div className="px-6 py-3 border-b border-border flex items-center gap-4 flex-wrap">
        {/* Search */}
        <div className="flex-1 min-w-64">
          <input
            type="text"
            placeholder="Search events..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full px-3 py-2 bg-bg-tertiary border border-border rounded-lg text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent-blue"
          />
        </div>
        
        {/* Type Filter */}
        <div className="flex items-center gap-2">
          <FunnelIcon className="w-4 h-4 text-text-muted" />
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value as FilterType)}
            className="px-3 py-2 bg-bg-tertiary border border-border rounded-lg text-sm text-text-primary focus:outline-none focus:border-accent-blue"
          >
            <option value="all">All Types</option>
            {eventTypes.map(type => (
              <option key={type} value={type}>{type.replace('_', ' ')}</option>
            ))}
          </select>
        </div>
      </div>
      
      {/* Event List */}
      <div className="divide-y divide-border max-h-[600px] overflow-y-auto">
        {filteredEvents.length === 0 ? (
          <div className="px-6 py-12 text-center text-text-muted">
            No events match your filters
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
                    <ChevronDownIcon className="w-4 h-4 text-text-muted" />
                  ) : (
                    <ChevronRightIcon className="w-4 h-4 text-text-muted" />
                  )}
                  
                  {/* Timestamp */}
                  <span className="text-xs font-mono text-text-muted w-20 flex-shrink-0">
                    {formatTimestamp(event.timestamp)}
                  </span>
                  
                  {/* Event Type Badge */}
                  <span className={clsx(
                    'px-2 py-0.5 rounded text-xs font-medium flex-shrink-0',
                    color
                  )}>
                    {parsed.title}
                  </span>
                  
                  {/* Process */}
                  <span className="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-muted flex-shrink-0">
                    {event.comm}:{event.pid}
                  </span>
                  
                  {/* Subtitle */}
                  <span className="text-sm text-text-secondary truncate flex-1 min-w-0">
                    {parsed.subtitle}
                  </span>
                </div>
                
                {/* Expanded Details */}
                {isExpanded && (
                  <div className="px-6 pb-4 pt-0">
                    <div className="ml-8 p-4 bg-bg-tertiary rounded-lg">
                      <div className="grid grid-cols-2 gap-4 mb-4">
                        <div>
                          <span className="text-xs text-text-muted block mb-1">Event ID</span>
                          <span className="text-xs font-mono text-text-secondary">{event.id}</span>
                        </div>
                        <div>
                          <span className="text-xs text-text-muted block mb-1">Timestamp</span>
                          <span className="text-xs font-mono text-text-secondary">
                            {new Date(event.timestamp).toISOString()}
                          </span>
                        </div>
                        <div>
                          <span className="text-xs text-text-muted block mb-1">Process</span>
                          <span className="text-xs font-mono text-text-secondary">
                            {event.comm} (PID: {event.pid}{event.ppid ? `, PPID: ${event.ppid}` : ''})
                          </span>
                        </div>
                        <div>
                          <span className="text-xs text-text-muted block mb-1">Type</span>
                          <span className="text-xs font-mono text-text-secondary">{event.type}</span>
                        </div>
                      </div>
                      
                      <div>
                        <span className="text-xs text-text-muted block mb-2">Data</span>
                        <pre className="p-3 bg-bg-primary rounded text-xs font-mono text-text-secondary overflow-x-auto">
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

