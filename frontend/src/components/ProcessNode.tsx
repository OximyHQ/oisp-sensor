'use client';

import { ProcessNode, TimelineItem } from '@/types/event';
import { countEventsByType } from '@/utils/eventParsers';
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
  const indent = depth * 24;
  
  // Count events by type
  const eventCounts = countEventsByType(node.events);
  
  return (
    <div className="animate-fade-in">
      {/* Process Header */}
      <div
        onClick={() => onToggleProcess(node.pid)}
        className={clsx(
          'flex items-center gap-3 py-2.5 px-3 rounded-lg cursor-pointer transition-all',
          'hover:bg-bg-tertiary group',
          depth > 0 && 'border-l-2 border-border ml-2'
        )}
        style={{ marginLeft: indent }}
      >
        {/* Expand/Collapse Icon */}
        {hasContent ? (
          isExpanded ? (
            <ChevronDownIcon className="w-4 h-4 text-text-muted flex-shrink-0 transition-transform" />
          ) : (
            <ChevronRightIcon className="w-4 h-4 text-text-muted flex-shrink-0 transition-transform" />
          )
        ) : (
          <div className="w-4" />
        )}
        
        {/* Process Icon */}
        <div className="w-8 h-8 rounded-lg bg-accent-purple/10 flex items-center justify-center flex-shrink-0">
          <CpuChipIcon className="w-4 h-4 text-accent-purple" />
        </div>
        
        {/* Process Info */}
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <span className="font-semibold text-sm text-text-primary truncate">
            {node.comm}
          </span>
          
          <span className="px-1.5 py-0.5 bg-bg-tertiary rounded text-[10px] font-mono text-text-muted group-hover:bg-bg-elevated">
            {node.pid}
          </span>
          
          {node.ppid && (
            <span className="text-[10px] text-text-muted hidden sm:inline">
              PPID {node.ppid}
            </span>
          )}
        </div>
        
        {/* Event Badges */}
        <div className="flex items-center gap-1.5 flex-shrink-0">
          {eventCounts.ai_prompt && eventCounts.ai_prompt > 0 && (
            <Badge color="green" count={eventCounts.ai_prompt} label="prompts" />
          )}
          {eventCounts.ai_response && eventCounts.ai_response > 0 && (
            <Badge color="blue" count={eventCounts.ai_response} label="responses" />
          )}
          {((eventCounts.file_open || 0) + (eventCounts.file_write || 0)) > 0 && (
            <Badge 
              color="cyan" 
              count={(eventCounts.file_open || 0) + (eventCounts.file_write || 0)} 
              label="files" 
            />
          )}
          {node.children.length > 0 && (
            <Badge color="purple" count={node.children.length} label="children" />
          )}
        </div>
      </div>
      
      {/* Expanded Content */}
      {isExpanded && hasContent && (
        <div 
          className="mt-0.5 space-y-0.5"
          style={{ marginLeft: indent + 28 }}
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
    green: 'bg-accent-green/10 text-accent-green',
    blue: 'bg-accent-blue/10 text-accent-blue',
    cyan: 'bg-accent-cyan/10 text-accent-cyan',
    purple: 'bg-accent-purple/10 text-accent-purple',
    orange: 'bg-accent-orange/10 text-accent-orange',
    red: 'bg-accent-red/10 text-accent-red',
  };
  
  return (
    <span 
      className={clsx(
        'px-1.5 py-0.5 rounded text-[10px] font-medium',
        colorClasses[color]
      )}
      title={`${count} ${label}`}
    >
      {count}
    </span>
  );
}
