'use client';

import { ParsedEvent } from '@/types/event';
import { formatTimestamp, getEventTypeColor } from '@/utils/eventParsers';
import { 
  ChevronRightIcon, 
  ChevronDownIcon,
  ArrowUpCircleIcon,
  ArrowDownCircleIcon,
  DocumentIcon,
  PlayIcon,
  StopIcon,
  GlobeAltIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

interface EventBlockProps {
  event: ParsedEvent;
  isExpanded: boolean;
  onToggle: () => void;
}

export function EventBlock({ event, isExpanded, onToggle }: EventBlockProps) {
  const colorClass = getEventTypeColor(event.type);
  const Icon = getEventIcon(event.type);
  
  return (
    <div className="animate-slide-in">
      {/* Event Header */}
      <div
        onClick={onToggle}
        className={clsx(
          'flex items-center gap-3 py-2 px-3 rounded-lg cursor-pointer transition-colors',
          'hover:bg-bg-tertiary border-l-2',
          event.type === 'ai_prompt' && 'border-accent-green',
          event.type === 'ai_response' && 'border-accent-blue',
          event.type === 'file_open' && 'border-accent-cyan',
          event.type === 'file_write' && 'border-accent-cyan',
          event.type === 'process_exec' && 'border-accent-purple',
          event.type === 'process_exit' && 'border-accent-red',
          event.type === 'network_connect' && 'border-accent-orange',
          !['ai_prompt', 'ai_response', 'file_open', 'file_write', 'process_exec', 'process_exit', 'network_connect'].includes(event.type) && 'border-border'
        )}
      >
        {/* Expand Icon */}
        {isExpanded ? (
          <ChevronDownIcon className="w-3 h-3 text-text-muted flex-shrink-0" />
        ) : (
          <ChevronRightIcon className="w-3 h-3 text-text-muted flex-shrink-0" />
        )}
        
        {/* Event Type Icon */}
        <div className={clsx('w-6 h-6 rounded flex items-center justify-center flex-shrink-0', colorClass)}>
          <Icon className="w-3.5 h-3.5" />
        </div>
        
        {/* Event Title & Subtitle */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className={clsx('font-mono text-sm font-medium', colorClass.split(' ')[0])}>
              {event.title}
            </span>
            {event.subtitle && (
              <span className="text-sm text-text-secondary truncate">
                {event.subtitle}
              </span>
            )}
          </div>
        </div>
        
        {/* Timestamp */}
        <span className="text-xs font-mono text-text-muted flex-shrink-0">
          {formatTimestamp(event.timestamp)}
        </span>
      </div>
      
      {/* Expanded Content */}
      {isExpanded && (
        <div className="ml-9 mt-1 mb-2">
          <pre className="p-3 bg-bg-tertiary rounded-lg text-xs font-mono text-text-secondary overflow-x-auto">
            {JSON.stringify(event.data, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}

function getEventIcon(type: string) {
  switch (type) {
    case 'ai_prompt':
      return ArrowUpCircleIcon;
    case 'ai_response':
      return ArrowDownCircleIcon;
    case 'file_open':
    case 'file_write':
      return DocumentIcon;
    case 'process_exec':
      return PlayIcon;
    case 'process_exit':
      return StopIcon;
    case 'network_connect':
      return GlobeAltIcon;
    default:
      return DocumentIcon;
  }
}

