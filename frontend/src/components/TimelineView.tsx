'use client';

import { useMemo, useState } from 'react';
import { WebEvent } from '@/types/event';
import { formatTimestamp, getEventTypeColor, parseEvent } from '@/utils/eventParsers';
import clsx from 'clsx';

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
      <div className="bg-bg-secondary rounded-xl border border-border p-8 text-center">
        <p className="text-text-secondary">No events to display</p>
      </div>
    );
  }
  
  return (
    <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-text-primary">
            Timeline View
          </h2>
          <p className="text-sm text-text-muted mt-1">
            Chronological event stream across all processes
          </p>
        </div>
        
        <div className="flex items-center gap-4">
          {/* Zoom Controls */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setZoomLevel(Math.max(50, zoomLevel - 25))}
              className="px-2 py-1 text-xs bg-bg-tertiary hover:bg-bg-hover rounded transition-colors"
            >
              -
            </button>
            <span className="text-xs text-text-muted w-12 text-center">
              {zoomLevel}%
            </span>
            <button
              onClick={() => setZoomLevel(Math.min(200, zoomLevel + 25))}
              className="px-2 py-1 text-xs bg-bg-tertiary hover:bg-bg-hover rounded transition-colors"
            >
              +
            </button>
          </div>
          
          {/* Duration */}
          <div className="text-sm text-text-muted">
            Duration: {formatDuration(timeRange.duration)} - {events.length} events
          </div>
        </div>
      </div>
      
      {/* Legend */}
      <div className="px-6 py-3 border-b border-border flex items-center gap-4 flex-wrap">
        {sources.map(source => (
          <div key={source} className="flex items-center gap-2">
            <div className={clsx(
              'w-3 h-3 rounded-sm',
              getEventTypeColor(source).split(' ')[1]
            )} />
            <span className="text-xs text-text-secondary">{source.replace('_', ' ')}</span>
          </div>
        ))}
      </div>
      
      {/* Timeline Visualization */}
      <div className="p-6">
        {/* Time axis */}
        <div className="flex items-center gap-2 mb-4 text-xs text-text-muted">
          <span>{formatTimestamp(timeRange.start)}</span>
          <div className="flex-1 h-px bg-border" />
          <span>{formatTimestamp(timeRange.end)}</span>
        </div>
        
        {/* Event rows by type */}
        <div className="space-y-4">
          {sources.map(source => {
            const sourceEvents = events.filter(e => e.type === source);
            const color = getEventTypeColor(source);
            
            return (
              <div key={source} className="flex items-center gap-4">
                <div className="w-28 text-xs text-text-secondary truncate">
                  {source.replace('_', ' ')}
                </div>
                <div className="flex-1 h-8 bg-bg-tertiary rounded relative overflow-hidden">
                  {sourceEvents.map((event, idx) => {
                    const position = timeRange.duration > 0
                      ? ((event.timestamp - timeRange.start) / timeRange.duration) * 100
                      : 50;
                    
                    return (
                      <div
                        key={event.id}
                        className={clsx(
                          'absolute top-1 bottom-1 w-1.5 rounded-sm transition-all hover:w-3',
                          color.split(' ')[1]
                        )}
                        style={{ left: `${position}%` }}
                        title={`${formatTimestamp(event.timestamp)} - ${event.comm}`}
                      />
                    );
                  })}
                </div>
                <div className="w-12 text-xs text-text-muted text-right">
                  {sourceEvents.length}
                </div>
              </div>
            );
          })}
        </div>
      </div>
      
      {/* Time-grouped Events List */}
      <div className="border-t border-border">
        <div className="px-6 py-3 bg-bg-tertiary/50 text-xs font-medium text-text-muted uppercase tracking-wide">
          Event Log
        </div>
        <div className="max-h-96 overflow-y-auto">
          {timelineGroups.map(group => (
            <div key={group.timestamp} className="border-b border-border last:border-0">
              <div className="px-6 py-2 bg-bg-tertiary/30 text-xs font-mono text-text-muted sticky top-0">
                {group.label}
              </div>
              {group.events.map(event => {
                const parsed = parseEvent(event);
                const color = getEventTypeColor(event.type);
                
                return (
                  <div 
                    key={event.id}
                    className="px-6 py-2 flex items-center gap-4 hover:bg-bg-tertiary/50 transition-colors"
                  >
                    <span className="text-xs font-mono text-text-muted w-20">
                      {formatTimestamp(event.timestamp)}
                    </span>
                    <span className={clsx(
                      'px-2 py-0.5 rounded text-xs font-medium',
                      color
                    )}>
                      {parsed.title}
                    </span>
                    <span className="text-xs text-text-muted truncate flex-1">
                      {parsed.subtitle}
                    </span>
                    <span className="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-muted">
                      {event.comm}
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

