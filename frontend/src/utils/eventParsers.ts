/**
 * Event parsing utilities
 * 
 * Core algorithm from AgentSight: Group events by PID, build parent-child hierarchy
 */

import {
  WebEvent,
  WebEventType,
  ParsedEvent,
  ProcessNode,
  TimelineItem,
  isAiPromptData,
  isAiResponseData,
  isProcessExecData,
  isProcessExitData,
  isFileOpData,
} from '@/types/event';

/**
 * Parse a WebEvent into a display-friendly ParsedEvent
 */
export function parseEvent(event: WebEvent): ParsedEvent {
  const { title, subtitle } = getEventTitleAndSubtitle(event);
  
  return {
    id: event.id,
    timestamp: event.timestamp,
    type: event.type,
    title,
    subtitle,
    data: event.data,
    isExpanded: false,
  };
}

/**
 * Get display title and subtitle for an event
 */
function getEventTitleAndSubtitle(event: WebEvent): { title: string; subtitle?: string } {
  const { type, data } = event;
  
  switch (type) {
    case 'ai_prompt':
      if (isAiPromptData(data)) {
        return {
          title: `AI PROMPT`,
          subtitle: `${data.model} - ${data.message_count} messages${data.streaming ? ' (streaming)' : ''}`,
        };
      }
      break;
      
    case 'ai_response':
      if (isAiResponseData(data)) {
        const latency = data.latency_ms ? ` (${(data.latency_ms / 1000).toFixed(1)}s)` : '';
        const tokens = data.output_tokens ? ` - ${data.output_tokens} tokens` : '';
        return {
          title: `AI RESPONSE`,
          subtitle: `${data.model}${tokens}${latency}`,
        };
      }
      break;
      
    case 'process_exec':
      if (isProcessExecData(data)) {
        return {
          title: 'EXEC',
          subtitle: data.cmdline || data.exe || undefined,
        };
      }
      break;
      
    case 'process_exit':
      if (isProcessExitData(data)) {
        const duration = data.duration_ms ? ` (${(data.duration_ms / 1000).toFixed(1)}s)` : '';
        return {
          title: 'EXIT',
          subtitle: `code ${data.exit_code}${duration}`,
        };
      }
      break;
      
    case 'file_open':
    case 'file_write':
      if (isFileOpData(data)) {
        const bytes = data.bytes ? ` (${formatBytes(data.bytes)})` : '';
        return {
          title: `FILE_${data.operation.toUpperCase()}`,
          subtitle: `${data.path}${bytes}`,
        };
      }
      break;
      
    case 'network_connect':
      return {
        title: 'CONNECT',
        subtitle: `${(data as { remote_addr: string; remote_port: number }).remote_addr}:${(data as { remote_addr: string; remote_port: number }).remote_port}`,
      };
  }
  
  return { title: type.toUpperCase().replace('_', ' ') };
}

/**
 * Build a process tree from a list of events
 * 
 * Algorithm:
 * 1. Group events by PID
 * 2. Build parent-child relationships using ppid
 * 3. Create timeline mixing events and child process spawns
 * 4. Return root processes (those without parents in our data)
 */
export function buildProcessTree(events: WebEvent[]): ProcessNode[] {
  const processMap = new Map<number, ProcessNode>();
  
  // First pass: create process nodes and collect events
  for (const event of events) {
    const { pid, ppid, comm } = event;
    
    if (!processMap.has(pid)) {
      processMap.set(pid, {
        pid,
        ppid,
        comm,
        children: [],
        events: [],
        timeline: [],
        isExpanded: true, // Expand by default
      });
    }
    
    // Update ppid if we see it (some events might have it, some might not)
    const node = processMap.get(pid)!;
    if (ppid !== undefined && node.ppid === undefined) {
      node.ppid = ppid;
    }
    
    // Parse and add event
    node.events.push(parseEvent(event));
  }
  
  // Sort events within each process by timestamp
  for (const node of processMap.values()) {
    node.events.sort((a, b) => a.timestamp - b.timestamp);
  }
  
  // Second pass: build parent-child relationships
  const childPids = new Set<number>();
  
  for (const node of processMap.values()) {
    if (node.ppid !== undefined && processMap.has(node.ppid)) {
      const parent = processMap.get(node.ppid)!;
      parent.children.push(node);
      childPids.add(node.pid);
    }
  }
  
  // Sort children by earliest timestamp
  for (const node of processMap.values()) {
    node.children.sort((a, b) => getEarliestTimestamp(a) - getEarliestTimestamp(b));
  }
  
  // Third pass: build timeline (mix events and child spawns)
  for (const node of processMap.values()) {
    const items: TimelineItem[] = [];
    
    // Add events
    for (const event of node.events) {
      items.push({
        type: 'event',
        timestamp: event.timestamp,
        event,
      });
    }
    
    // Add child process spawns
    for (const child of node.children) {
      items.push({
        type: 'process',
        timestamp: getEarliestTimestamp(child),
        process: child,
      });
    }
    
    // Sort by timestamp
    items.sort((a, b) => a.timestamp - b.timestamp);
    node.timeline = items;
  }
  
  // Return root processes (those not claimed as children)
  const roots: ProcessNode[] = [];
  for (const [pid, node] of processMap) {
    if (!childPids.has(pid)) {
      roots.push(node);
    }
  }
  
  // Sort roots by earliest timestamp
  return roots.sort((a, b) => getEarliestTimestamp(a) - getEarliestTimestamp(b));
}

/**
 * Get the earliest timestamp from a process node (its events or children)
 */
function getEarliestTimestamp(node: ProcessNode): number {
  let earliest = Infinity;
  
  if (node.events.length > 0) {
    earliest = Math.min(earliest, node.events[0].timestamp);
  }
  
  for (const child of node.children) {
    earliest = Math.min(earliest, getEarliestTimestamp(child));
  }
  
  return earliest === Infinity ? 0 : earliest;
}

/**
 * Count total events in a process tree (including children)
 */
export function countEvents(node: ProcessNode): number {
  let count = node.events.length;
  for (const child of node.children) {
    count += countEvents(child);
  }
  return count;
}

/**
 * Count events by type for a process
 */
export function countEventsByType(events: ParsedEvent[]): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const event of events) {
    counts[event.type] = (counts[event.type] || 0) + 1;
  }
  return counts;
}

/**
 * Format bytes to human readable string
 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/**
 * Format timestamp to display string
 */
export function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
}

/**
 * Format relative time (e.g., "2s ago")
 */
export function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  
  if (diff < 1000) return 'just now';
  if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  return `${Math.floor(diff / 86400000)}d ago`;
}

/**
 * Get color class for event type
 */
export function getEventTypeColor(type: WebEventType): string {
  switch (type) {
    case 'ai_prompt':
      return 'text-accent-green bg-accent-green/10';
    case 'ai_response':
      return 'text-accent-blue bg-accent-blue/10';
    case 'process_exec':
      return 'text-accent-purple bg-accent-purple/10';
    case 'process_exit':
      return 'text-accent-red bg-accent-red/10';
    case 'file_open':
    case 'file_write':
      return 'text-accent-cyan bg-accent-cyan/10';
    case 'network_connect':
      return 'text-accent-orange bg-accent-orange/10';
    default:
      return 'text-text-secondary bg-bg-tertiary';
  }
}

/**
 * Get icon name for event type
 */
export function getEventTypeIcon(type: WebEventType): string {
  switch (type) {
    case 'ai_prompt':
      return 'arrow-up-circle';
    case 'ai_response':
      return 'arrow-down-circle';
    case 'process_exec':
      return 'play';
    case 'process_exit':
      return 'stop';
    case 'file_open':
    case 'file_write':
      return 'document';
    case 'network_connect':
      return 'globe';
    default:
      return 'question-mark-circle';
  }
}

