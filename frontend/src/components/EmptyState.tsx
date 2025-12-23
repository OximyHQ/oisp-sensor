'use client';

import {
  CpuChipIcon,
  ChatBubbleLeftRightIcon,
  ArrowPathIcon,
  DocumentTextIcon,
  CommandLineIcon,
} from '@heroicons/react/24/outline';

export function EmptyState() {
  return (
    <div className="bg-bg-secondary rounded-xl border border-border p-12">
      <div className="max-w-lg mx-auto text-center">
        {/* Icon */}
        <div className="relative w-20 h-20 mx-auto mb-6">
          <div className="absolute inset-0 bg-gradient-to-br from-accent-blue/20 to-accent-purple/20 rounded-2xl blur-xl" />
          <div className="relative w-full h-full rounded-2xl bg-gradient-to-br from-accent-blue/10 to-accent-purple/10 border border-accent-blue/20 flex items-center justify-center">
            <CpuChipIcon className="w-10 h-10 text-accent-blue" />
          </div>
        </div>
        
        {/* Title */}
        <h3 className="text-xl font-semibold text-text-primary mb-2">
          Waiting for Events
        </h3>
        
        <p className="text-text-secondary mb-8">
          Start your AI agent to see events flow in real-time. OISP Sensor captures 
          AI prompts, API calls, process activity, and file operations.
        </p>
        
        {/* Feature Cards */}
        <div className="grid grid-cols-2 gap-3 mb-8">
          <FeatureCard
            icon={ChatBubbleLeftRightIcon}
            title="AI Calls"
            description="Prompts & responses"
            color="green"
          />
          <FeatureCard
            icon={CpuChipIcon}
            title="Processes"
            description="Exec & exit events"
            color="purple"
          />
          <FeatureCard
            icon={DocumentTextIcon}
            title="File I/O"
            description="Reads & writes"
            color="cyan"
          />
          <FeatureCard
            icon={ArrowPathIcon}
            title="Traces"
            description="Agent sessions"
            color="orange"
          />
        </div>
        
        {/* Quick Start */}
        <div className="p-4 bg-bg-tertiary rounded-lg text-left">
          <div className="flex items-center gap-2 text-xs text-text-muted mb-2">
            <CommandLineIcon className="w-4 h-4" />
            <span>Quick Start</span>
          </div>
          <code className="block text-sm font-mono text-text-secondary">
            # Run a Python agent with OpenAI
          </code>
          <code className="block text-sm font-mono text-accent-blue mt-1">
            python your_agent.py
          </code>
        </div>
        
        {/* Loading indicator */}
        <div className="mt-8 flex items-center justify-center gap-2">
          <div className="flex gap-1">
            <div className="w-2 h-2 rounded-full bg-accent-blue animate-pulse" />
            <div className="w-2 h-2 rounded-full bg-accent-blue animate-pulse" style={{ animationDelay: '0.15s' }} />
            <div className="w-2 h-2 rounded-full bg-accent-blue animate-pulse" style={{ animationDelay: '0.3s' }} />
          </div>
          <span className="text-xs text-text-muted">Listening for events...</span>
        </div>
      </div>
    </div>
  );
}

function FeatureCard({
  icon: Icon,
  title,
  description,
  color,
}: {
  icon: typeof CpuChipIcon;
  title: string;
  description: string;
  color: 'green' | 'purple' | 'cyan' | 'orange';
}) {
  const colorClasses = {
    green: 'bg-accent-green/5 border-accent-green/20 text-accent-green',
    purple: 'bg-accent-purple/5 border-accent-purple/20 text-accent-purple',
    cyan: 'bg-accent-cyan/5 border-accent-cyan/20 text-accent-cyan',
    orange: 'bg-accent-orange/5 border-accent-orange/20 text-accent-orange',
  };
  
  return (
    <div className={`p-3 rounded-lg border ${colorClasses[color]}`}>
      <div className="flex items-center gap-2 mb-1">
        <Icon className="w-4 h-4" />
        <span className="text-sm font-medium text-text-primary">{title}</span>
      </div>
      <p className="text-xs text-text-muted">{description}</p>
    </div>
  );
}
