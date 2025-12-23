'use client';

import { useState, useCallback } from 'react';
import { ProcessNode } from '@/types/event';
import { ProcessNodeComponent } from './ProcessNode';
import {
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
} from '@heroicons/react/24/outline';

interface ProcessTreeViewProps {
  processTree: ProcessNode[];
}

export function ProcessTreeView({ processTree }: ProcessTreeViewProps) {
  const [expandedProcesses, setExpandedProcesses] = useState<Set<number>>(() => {
    // Expand all processes by default
    const expanded = new Set<number>();
    const addAll = (nodes: ProcessNode[]) => {
      for (const node of nodes) {
        expanded.add(node.pid);
        addAll(node.children);
      }
    };
    addAll(processTree);
    return expanded;
  });
  
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());
  
  const toggleProcess = useCallback((pid: number) => {
    setExpandedProcesses(prev => {
      const next = new Set(prev);
      if (next.has(pid)) {
        next.delete(pid);
      } else {
        next.add(pid);
      }
      return next;
    });
  }, []);
  
  const toggleEvent = useCallback((eventId: string) => {
    setExpandedEvents(prev => {
      const next = new Set(prev);
      if (next.has(eventId)) {
        next.delete(eventId);
      } else {
        next.add(eventId);
      }
      return next;
    });
  }, []);
  
  const expandAll = useCallback(() => {
    const expanded = new Set<number>();
    const addAll = (nodes: ProcessNode[]) => {
      for (const node of nodes) {
        expanded.add(node.pid);
        addAll(node.children);
      }
    };
    addAll(processTree);
    setExpandedProcesses(expanded);
  }, [processTree]);
  
  const collapseAll = useCallback(() => {
    setExpandedProcesses(new Set());
  }, []);

  // Calculate stats
  const stats = {
    processes: 0,
    events: 0,
  };
  
  const countAll = (nodes: ProcessNode[]) => {
    for (const node of nodes) {
      stats.processes++;
      stats.events += node.events.length;
      countAll(node.children);
    }
  };
  countAll(processTree);
  
  if (processTree.length === 0) {
    return (
      <div className="bg-bg-secondary rounded-xl border border-border p-12 text-center">
        <p className="text-text-secondary">No processes to display</p>
        <p className="text-sm text-text-muted mt-1">
          Process events will appear here as your agents run
        </p>
      </div>
    );
  }
  
  return (
    <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold text-text-primary">
            Process Tree
          </h2>
          <p className="text-xs text-text-muted mt-0.5">
            {stats.processes} process{stats.processes !== 1 ? 'es' : ''} with {stats.events} event{stats.events !== 1 ? 's' : ''}
          </p>
        </div>
        
        <div className="flex items-center gap-2">
          <button
            onClick={expandAll}
            className="btn btn-ghost text-xs"
            title="Expand all"
          >
            <ChevronDoubleDownIcon className="w-4 h-4" />
            Expand All
          </button>
          <button
            onClick={collapseAll}
            className="btn btn-ghost text-xs"
            title="Collapse all"
          >
            <ChevronDoubleUpIcon className="w-4 h-4" />
            Collapse All
          </button>
        </div>
      </div>
      
      {/* Process Tree */}
      <div className="p-4">
        <div className="space-y-1">
          {processTree.map(process => (
            <ProcessNodeComponent
              key={process.pid}
              node={process}
              depth={0}
              expandedProcesses={expandedProcesses}
              expandedEvents={expandedEvents}
              onToggleProcess={toggleProcess}
              onToggleEvent={toggleEvent}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
