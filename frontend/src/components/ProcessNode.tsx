'use client';

import { ProcessNode, TimelineItem } from '@/types/event';
import { countEventsByType, formatTimestamp } from '@/utils/eventParsers';
import { EventBlock } from './EventBlock';
import { 
  ChevronRightIcon, 
  ChevronDownIcon,
  CpuChipIcon
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

interface ProcessNodeComponentProps {
  node: ProcessNode;
  depth: number;
  expandedProcesses: Set<number>;
  expandedEvents: Set<string>;
  onToggleProcess: (pid: number) => void;
  onToggleEvent: (eventId: string) => void;
}

export function ProcessNodeComponent({
  node,
  depth,
  expandedProcesses,
  expandedEvents,
  onToggleProcess,
  onToggleEvent,
}: ProcessNodeComponentProps) {
  const isExpanded = expandedProcesses.has(node.pid);
  const hasContent = node.events.length > 0 || node.children.length > 0;
  const indent = depth * 28;
  
  // Count events by type
  const eventCounts = countEventsByType(node.events);
  
  return (
    <div className="animate-fade-in">
      {/* Process Header */}
      <div
        onClick={() => onToggleProcess(node.pid)}
        className={clsx(
          'flex items-center gap-3 py-3 px-4 rounded-lg cursor-pointer transition-colors',
          'hover:bg-bg-tertiary',
          depth > 0 && 'ml-2 border-l-2 border-border'
        )}
        style={{ marginLeft: indent }}
      >
        {/* Expand/Collapse Icon */}
        {hasContent ? (
          isExpanded ? (
            <ChevronDownIcon className="w-4 h-4 text-text-muted flex-shrink-0" />
          ) : (
            <ChevronRightIcon className="w-4 h-4 text-text-muted flex-shrink-0" />
          )
        ) : (
          <div className="w-4" />
        )}
        
        {/* Process Icon */}
        <div className="w-8 h-8 rounded-lg bg-accent-purple/20 flex items-center justify-center flex-shrink-0">
          <CpuChipIcon className="w-4 h-4 text-accent-purple" />
        </div>
        
        {/* Process Info */}
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <span className="px-2 py-0.5 bg-bg-tertiary rounded text-xs font-mono text-text-muted">
            PID {node.pid}
          </span>
          
          <span className="font-semibold text-text-primary truncate">
            [{node.comm}]
          </span>
          
          {node.ppid && (
            <span className="text-xs text-text-muted">
              &larr; {node.ppid}
            </span>
          )}
        </div>
        
        {/* Event Badges */}
        <div className="flex items-center gap-2 flex-shrink-0">
          {eventCounts.ai_prompt && (
            <Badge color="green" count={eventCounts.ai_prompt} label="prompts" />
          )}
          {eventCounts.ai_response && (
            <Badge color="blue" count={eventCounts.ai_response} label="responses" />
          )}
          {(eventCounts.file_open || eventCounts.file_write) && (
            <Badge 
              color="cyan" 
              count={(eventCounts.file_open || 0) + (eventCounts.file_write || 0)} 
              label="files" 
            />
          )}
          {(eventCounts.process_exec || eventCounts.process_exit) && (
            <Badge 
              color="purple" 
              count={(eventCounts.process_exec || 0) + (eventCounts.process_exit || 0)} 
              label="process" 
            />
          )}
        </div>
      </div>
      
      {/* Expanded Content */}
      {isExpanded && hasContent && (
        <div 
          className="mt-1 space-y-1"
          style={{ marginLeft: indent + 32 }}
        >
          {node.timeline.map((item, index) => (
            <TimelineItemComponent
              key={`${item.type}-${item.timestamp}-${index}`}
              item={item}
              depth={depth}
              expandedProcesses={expandedProcesses}
              expandedEvents={expandedEvents}
              onToggleProcess={onToggleProcess}
              onToggleEvent={onToggleEvent}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function TimelineItemComponent({
  item,
  depth,
  expandedProcesses,
  expandedEvents,
  onToggleProcess,
  onToggleEvent,
}: {
  item: TimelineItem;
  depth: number;
  expandedProcesses: Set<number>;
  expandedEvents: Set<string>;
  onToggleProcess: (pid: number) => void;
  onToggleEvent: (eventId: string) => void;
}) {
  if (item.type === 'event' && item.event) {
    return (
      <EventBlock
        event={item.event}
        isExpanded={expandedEvents.has(item.event.id)}
        onToggle={() => onToggleEvent(item.event!.id)}
      />
    );
  }
  
  if (item.type === 'process' && item.process) {
    return (
      <ProcessNodeComponent
        node={item.process}
        depth={depth + 1}
        expandedProcesses={expandedProcesses}
        expandedEvents={expandedEvents}
        onToggleProcess={onToggleProcess}
        onToggleEvent={onToggleEvent}
      />
    );
  }
  
  return null;
}

function Badge({ 
  color, 
  count, 
  label 
}: { 
  color: 'green' | 'blue' | 'cyan' | 'purple' | 'orange' | 'red';
  count: number; 
  label: string;
}) {
  const colorClasses = {
    green: 'bg-accent-green/20 text-accent-green',
    blue: 'bg-accent-blue/20 text-accent-blue',
    cyan: 'bg-accent-cyan/20 text-accent-cyan',
    purple: 'bg-accent-purple/20 text-accent-purple',
    orange: 'bg-accent-orange/20 text-accent-orange',
    red: 'bg-accent-red/20 text-accent-red',
  };
  
  return (
    <span className={clsx(
      'px-2 py-0.5 rounded-full text-xs font-medium',
      colorClasses[color]
    )}>
      {count} {count === 1 ? label.replace(/s$/, '') : label}
    </span>
  );
}

