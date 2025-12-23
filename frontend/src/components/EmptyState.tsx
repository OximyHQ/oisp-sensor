'use client';

import { CpuChipIcon } from '@heroicons/react/24/outline';

export function EmptyState() {
  return (
    <div className="bg-bg-secondary rounded-xl border border-border p-12 text-center">
      <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-accent-blue/20 to-accent-purple/20 flex items-center justify-center mx-auto mb-6">
        <CpuChipIcon className="w-8 h-8 text-accent-blue" />
      </div>
      
      <h3 className="text-xl font-semibold text-text-primary mb-2">
        No Events Yet
      </h3>
      
      <p className="text-text-secondary max-w-md mx-auto mb-8">
        Start your AI agent and watch the events flow in. 
        OISP Sensor captures AI prompts, API calls, file operations, and more.
      </p>
      
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 max-w-2xl mx-auto">
        <FeatureCard
          title="AI Prompts"
          description="Capture all AI requests and responses"
          color="green"
        />
        <FeatureCard
          title="Process Tree"
          description="See parent-child process relationships"
          color="purple"
        />
        <FeatureCard
          title="File Operations"
          description="Track file reads, writes, and opens"
          color="cyan"
        />
      </div>
      
      <div className="mt-8 p-4 bg-bg-tertiary rounded-lg max-w-lg mx-auto">
        <p className="text-sm text-text-muted mb-2">Waiting for events...</p>
        <div className="flex items-center justify-center gap-2">
          <div className="w-2 h-2 rounded-full bg-accent-blue animate-pulse" />
          <div className="w-2 h-2 rounded-full bg-accent-blue animate-pulse" style={{ animationDelay: '0.2s' }} />
          <div className="w-2 h-2 rounded-full bg-accent-blue animate-pulse" style={{ animationDelay: '0.4s' }} />
        </div>
      </div>
    </div>
  );
}

function FeatureCard({
  title,
  description,
  color,
}: {
  title: string;
  description: string;
  color: 'green' | 'purple' | 'cyan' | 'blue';
}) {
  const colorClasses = {
    green: 'border-accent-green/30 bg-accent-green/5',
    purple: 'border-accent-purple/30 bg-accent-purple/5',
    cyan: 'border-accent-cyan/30 bg-accent-cyan/5',
    blue: 'border-accent-blue/30 bg-accent-blue/5',
  };
  
  return (
    <div className={`p-4 rounded-lg border ${colorClasses[color]}`}>
      <h4 className="font-medium text-text-primary text-sm mb-1">{title}</h4>
      <p className="text-xs text-text-muted">{description}</p>
    </div>
  );
}

