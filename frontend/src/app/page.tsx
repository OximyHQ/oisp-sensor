'use client';

import { useState, useMemo } from 'react';
import { useEvents } from '@/lib/useEvents';
import { useStats } from '@/lib/useStats';
import { useInventory } from '@/lib/useInventory';
import { useTraces } from '@/lib/useTraces';
import { buildProcessTree } from '@/utils/eventParsers';
import { Sidebar } from '@/components/Sidebar';
import { TopBar } from '@/components/TopBar';
import { DashboardView } from '@/components/DashboardView';
import { ProcessTreeView } from '@/components/ProcessTreeView';
import { TimelineView } from '@/components/TimelineView';
import { LogView } from '@/components/LogView';
import { InventoryView } from '@/components/InventoryView';
import { TracesView } from '@/components/TracesView';
import { SettingsView } from '@/components/SettingsView';
import { MetricsView } from '@/components/MetricsView';
import { EmptyState } from '@/components/EmptyState';

export type ViewType = 'dashboard' | 'tree' | 'timeline' | 'log' | 'inventory' | 'traces' | 'metrics' | 'settings';

export default function Home() {
  const [view, setView] = useState<ViewType>('dashboard');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  
  const { events, loading, error, connected, refresh } = useEvents();
  const { stats, loading: statsLoading } = useStats();
  const { inventory, loading: inventoryLoading, refresh: refreshInventory } = useInventory();
  const { traces, loading: tracesLoading, refresh: refreshTraces } = useTraces();
  
  // Build process tree from events
  const processTree = useMemo(() => {
    return buildProcessTree(events);
  }, [events]);
  
  // Compute event stats for the sidebar
  const eventStats = useMemo(() => {
    const aiPrompts = events.filter(e => e.type === 'ai_prompt').length;
    const aiResponses = events.filter(e => e.type === 'ai_response').length;
    const processes = new Set(events.map(e => e.pid)).size;
    
    return { total: events.length, aiPrompts, aiResponses, processes };
  }, [events]);
  
  const renderView = () => {
    if (loading && events.length === 0) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="w-8 h-8 border-2 border-accent-blue border-t-transparent rounded-full animate-spin mx-auto mb-4" />
            <p className="text-text-secondary">Loading events...</p>
          </div>
        </div>
      );
    }

    switch (view) {
      case 'dashboard':
        return (
          <DashboardView 
            events={events}
            stats={stats}
            inventory={inventory}
            traces={traces}
            connected={connected}
            onNavigate={setView}
          />
        );
      
      case 'tree':
        return events.length === 0 ? <EmptyState /> : <ProcessTreeView processTree={processTree} />;
      
      case 'timeline':
        return events.length === 0 ? <EmptyState /> : <TimelineView events={events} />;
      
      case 'log':
        return events.length === 0 ? <EmptyState /> : <LogView events={events} />;
      
      case 'inventory':
        return (
          <InventoryView 
            inventory={inventory} 
            loading={inventoryLoading} 
            onRefresh={refreshInventory}
          />
        );
      
      case 'traces':
        return (
          <TracesView 
            traces={traces} 
            loading={tracesLoading}
            onRefresh={refreshTraces}
          />
        );
      
      case 'metrics':
        return <MetricsView />;
      
      case 'settings':
        return <SettingsView />;
      
      default:
        return <EmptyState />;
    }
  };
  
  return (
    <div className="flex h-screen overflow-hidden">
      {/* Sidebar */}
      <Sidebar
        currentView={view}
        onViewChange={setView}
        collapsed={sidebarCollapsed}
        onToggleCollapse={() => setSidebarCollapsed(!sidebarCollapsed)}
        stats={eventStats}
        connected={connected}
      />
      
      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Top Bar */}
        <TopBar
          connected={connected}
          stats={stats}
          onRefresh={refresh}
          currentView={view}
        />
        
        {/* Main Content Area */}
        <main className="flex-1 overflow-auto p-6 bg-bg-primary">
          {error && (
            <div className="mb-4 p-4 bg-accent-red/10 border border-accent-red/30 rounded-lg text-accent-red text-sm">
              {error}
            </div>
          )}
          
          {renderView()}
        </main>
      </div>
    </div>
  );
}
