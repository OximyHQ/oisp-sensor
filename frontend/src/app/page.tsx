'use client';

import { useState, useMemo } from 'react';
import { useEvents } from '@/lib/useEvents';
import { buildProcessTree } from '@/utils/eventParsers';
import { Header } from '@/components/Header';
import { Navigation } from '@/components/Navigation';
import { ProcessTreeView } from '@/components/ProcessTreeView';
import { TimelineView } from '@/components/TimelineView';
import { LogView } from '@/components/LogView';
import { EmptyState } from '@/components/EmptyState';

type ViewType = 'tree' | 'timeline' | 'log';

export default function Home() {
  const [view, setView] = useState<ViewType>('tree');
  const { events, loading, error, connected, refresh } = useEvents();
  
  // Build process tree from events
  const processTree = useMemo(() => {
    return buildProcessTree(events);
  }, [events]);
  
  // Count stats
  const stats = useMemo(() => {
    const aiPrompts = events.filter(e => e.type === 'ai_prompt').length;
    const aiResponses = events.filter(e => e.type === 'ai_response').length;
    const processes = new Set(events.map(e => e.pid)).size;
    
    return { total: events.length, aiPrompts, aiResponses, processes };
  }, [events]);
  
  return (
    <div className="min-h-screen bg-bg-primary">
      <Header 
        stats={stats} 
        connected={connected} 
        onRefresh={refresh}
      />
      
      <Navigation 
        currentView={view} 
        onViewChange={setView}
        eventCount={events.length}
      />
      
      <main className="max-w-[1800px] mx-auto px-4 pb-8">
        {error && (
          <div className="mb-4 p-4 bg-accent-red/10 border border-accent-red/30 rounded-lg text-accent-red">
            {error}
          </div>
        )}
        
        {loading && events.length === 0 ? (
          <div className="flex items-center justify-center h-64">
            <div className="text-text-secondary">Loading events...</div>
          </div>
        ) : events.length === 0 ? (
          <EmptyState />
        ) : (
          <>
            {view === 'tree' && (
              <ProcessTreeView processTree={processTree} />
            )}
            
            {view === 'timeline' && (
              <TimelineView events={events} />
            )}
            
            {view === 'log' && (
              <LogView events={events} />
            )}
          </>
        )}
      </main>
    </div>
  );
}

