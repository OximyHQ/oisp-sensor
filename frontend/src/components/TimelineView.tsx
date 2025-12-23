'use client';

import { useMemo, useState } from 'react';
import { WebEvent } from '@/types/event';
import { formatTimestamp, getEventTypeColor, parseEvent } from '@/utils/eventParsers';
import clsx from 'clsx';
import {
  MinusIcon,
  PlusIcon,
  ClockIcon,
} from '@heroicons/react/24/outline';

interface TimelineViewProps {
  events: WebEvent[];
}

interface TimelineGroup {
  label: string;
  events: WebEvent[];
  timestamp: number;
}

export function TimelineView({ events }: TimelineViewProps) {
  const [zoomLevel, setZoomLevel] = useState(100);
  
  // Group events by time buckets (1 minute intervals)
  const timelineGroups = useMemo(() => {
    if (events.length === 0) return [];
    
    const sorted = [...events].sort((a, b) => a.timestamp - b.timestamp);
    const groups: TimelineGroup[] = [];
    
    let currentGroup: TimelineGroup | null = null;
    const bucketSize = 60000; // 1 minute
    
    for (const event of sorted) {
      const bucketTime = Math.floor(event.timestamp / bucketSize) * bucketSize;
      
      if (!currentGroup || currentGroup.timestamp !== bucketTime) {
        currentGroup = {
          label: formatTimestamp(bucketTime),
          events: [],
          timestamp: bucketTime,
        };
        groups.push(currentGroup);
      }
      
      currentGroup.events.push(event);
    }
    
    return groups;
  }, [events]);
  
  // Calculate unique sources for legend
  const sources = useMemo(() => {
    const sourceSet = new Set(events.map(e => e.type));
    return Array.from(sourceSet);
  }, [events]);
  
  // Get time range
  const timeRange = useMemo(() => {
    if (events.length === 0) return { start: 0, end: 0, duration: 0 };
    const sorted = [...events].sort((a, b) => a.timestamp - b.timestamp);
    const start = sorted[0].timestamp;
    const end = sorted[sorted.length - 1].timestamp;
    return { start, end, duration: end - start };
  }, [events]);
  
  if (events.length === 0) {
    return (
      <div className="bg-bg-secondary rounded-xl border border-border p-12 text-center">
        <ClockIcon className="w-12 h-12 text-text-muted mx-auto mb-4" />
        <p className="text-text-secondary">No events to display</p>
        <p className="text-sm text-text-muted mt-1">
          Timeline will populate as events arrive
        </p>
      </div>
    );
  }
  
  return (
    <div className="space-y-6">
      {/* Timeline Visualization */}
      <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
        <div className="px-6 py-4 border-b border-border flex items-center justify-between">
          <div>
            <h2 className="text-base font-semibold text-text-primary">
              Event Timeline
            </h2>
            <p className="text-xs text-text-muted mt-0.5">
              Duration: {formatDuration(timeRange.duration)} - {events.length} events
            </p>
          </div>
          
          <div className="flex items-center gap-4">
            {/* Zoom Controls */}
            <div className="flex items-center gap-1 bg-bg-tertiary rounded-lg p-1">
              <button
                onClick={() => setZoomLevel(Math.max(50, zoomLevel - 25))}
                className="p-1.5 rounded hover:bg-bg-hover transition-colors text-text-muted hover:text-text-primary"
              >
                <MinusIcon className="w-4 h-4" />
              </button>
              <span className="text-xs text-text-muted w-12 text-center font-mono">
                {zoomLevel}%
              </span>
              <button
                onClick={() => setZoomLevel(Math.min(200, zoomLevel + 25))}
                className="p-1.5 rounded hover:bg-bg-hover transition-colors text-text-muted hover:text-text-primary"
              >
                <PlusIcon className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
        
        {/* Legend */}
        <div className="px-6 py-3 border-b border-border flex items-center gap-4 flex-wrap bg-bg-tertiary/30">
          {sources.map(source => (
            <div key={source} className="flex items-center gap-2">
              <div className={clsx(
                'w-2.5 h-2.5 rounded-sm',
                getEventTypeColor(source).split(' ')[1]
              )} />
              <span className="text-xs text-text-secondary capitalize">{source.replace('_', ' ')}</span>
            </div>
          ))}
        </div>
        
        {/* Timeline Visualization */}
        <div className="p-6">
          {/* Time axis */}
          <div className="flex items-center gap-2 mb-4 text-[10px] font-mono text-text-muted">
            <span>{formatTimestamp(timeRange.start)}</span>
            <div className="flex-1 h-px bg-border" />
            <span>{formatTimestamp(timeRange.end)}</span>
          </div>
          
          {/* Event rows by type */}
          <div className="space-y-3">
            {sources.map(source => {
              const sourceEvents = events.filter(e => e.type === source);
              const color = getEventTypeColor(source);
              
              return (
                <div key={source} className="flex items-center gap-4">
                  <div className="w-24 text-xs text-text-secondary truncate capitalize">
                    {source.replace('_', ' ')}
                  </div>
                  <div className="flex-1 h-8 bg-bg-tertiary rounded-lg relative overflow-hidden">
                    {sourceEvents.map((event) => {
                      const position = timeRange.duration > 0
                        ? ((event.timestamp - timeRange.start) / timeRange.duration) * 100
                        : 50;
                      
                      return (
                        <div
                          key={event.id}
                          className={clsx(
                            'absolute top-1 bottom-1 w-1 rounded-sm transition-all hover:w-2 cursor-pointer',
                            color.split(' ')[1]
                          )}
                          style={{ left: `${Math.min(Math.max(position, 0), 99)}%` }}
                          title={`${formatTimestamp(event.timestamp)} - ${event.comm}`}
                        />
                      );
                    })}
                  </div>
                  <div className="w-10 text-xs font-mono text-text-muted text-right">
                    {sourceEvents.length}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
      
      {/* Time-grouped Events List */}
      <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
        <div className="px-6 py-4 border-b border-border">
          <h2 className="text-base font-semibold text-text-primary">Event Log</h2>
          <p className="text-xs text-text-muted mt-0.5">Grouped by minute</p>
        </div>
        
        <div className="max-h-[500px] overflow-y-auto">
          {timelineGroups.map(group => (
            <div key={group.timestamp} className="border-b border-border last:border-0">
              <div className="px-6 py-2 bg-bg-tertiary/50 text-xs font-mono text-text-muted sticky top-0 flex items-center gap-2">
                <ClockIcon className="w-3 h-3" />
                {group.label}
                <span className="text-text-muted">({group.events.length} events)</span>
              </div>
              {group.events.map(event => {
                const parsed = parseEvent(event);
                const color = getEventTypeColor(event.type);
                
                return (
                  <div 
                    key={event.id}
                    className="px-6 py-2.5 flex items-center gap-4 hover:bg-bg-tertiary/30 transition-colors"
                  >
                    <span className="text-[10px] font-mono text-text-muted w-16">
                      {formatTimestamp(event.timestamp)}
                    </span>
                    <span className={clsx(
                      'px-2 py-0.5 rounded text-[10px] font-medium',
                      color
                    )}>
                      {parsed.title}
                    </span>
                    <span className="text-xs text-text-muted truncate flex-1">
                      {parsed.subtitle}
                    </span>
                    <span className="px-2 py-0.5 bg-bg-tertiary rounded text-[10px] font-mono text-text-muted">
                      {event.comm}:{event.pid}
                    </span>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
  return `${(ms / 3600000).toFixed(1)}h`;
}
