'use client';

import { useState } from 'react';
import clsx from 'clsx';
import {
  CloudArrowUpIcon,
  DocumentTextIcon,
  GlobeAltIcon,
  ServerStackIcon,
  ShieldCheckIcon,
  CheckCircleIcon,
  XCircleIcon,
  ArrowPathIcon,
  InformationCircleIcon,
  EyeSlashIcon,
  Cog6ToothIcon,
} from '@heroicons/react/24/outline';

type SettingsTab = 'sinks' | 'redaction' | 'general';

// Sink configurations
interface SinkConfig {
  enabled: boolean;
}

interface JsonlConfig extends SinkConfig {
  path: string;
}

interface OtlpConfig extends SinkConfig {
  endpoint: string;
  protocol: 'grpc' | 'http/proto' | 'http/json';
  headers: Record<string, string>;
  compression: boolean;
}

interface KafkaConfig extends SinkConfig {
  brokers: string;
  topic: string;
  sasl_mechanism: 'none' | 'plain' | 'scram-sha-256' | 'scram-sha-512';
  username: string;
  password: string;
  compression: 'none' | 'gzip' | 'snappy' | 'lz4' | 'zstd';
}

interface WebhookConfig extends SinkConfig {
  url: string;
  method: 'POST' | 'PUT' | 'PATCH';
  auth_type: 'none' | 'api_key' | 'bearer' | 'basic';
  auth_value: string;
  batch_mode: boolean;
  max_retries: number;
}

interface RedactionConfig {
  mode: 'safe' | 'full' | 'minimal';
  redact_api_keys: boolean;
  redact_emails: boolean;
  redact_credit_cards: boolean;
  custom_patterns: string[];
}

export function SettingsView() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('sinks');
  const [saving, setSaving] = useState(false);
  const [testingConnection, setTestingConnection] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, 'success' | 'error' | null>>({});

  // Sink configs (in a real app, these would come from the API)
  const [jsonlConfig, setJsonlConfig] = useState<JsonlConfig>({
    enabled: true,
    path: '/var/log/oisp-sensor/events.jsonl',
  });

  const [otlpConfig, setOtlpConfig] = useState<OtlpConfig>({
    enabled: false,
    endpoint: 'http://localhost:4317',
    protocol: 'grpc',
    headers: {},
    compression: true,
  });

  const [kafkaConfig, setKafkaConfig] = useState<KafkaConfig>({
    enabled: false,
    brokers: 'localhost:9092',
    topic: 'oisp-events',
    sasl_mechanism: 'none',
    username: '',
    password: '',
    compression: 'gzip',
  });

  const [webhookConfig, setWebhookConfig] = useState<WebhookConfig>({
    enabled: false,
    url: '',
    method: 'POST',
    auth_type: 'none',
    auth_value: '',
    batch_mode: true,
    max_retries: 3,
  });

  const [redactionConfig, setRedactionConfig] = useState<RedactionConfig>({
    mode: 'safe',
    redact_api_keys: true,
    redact_emails: true,
    redact_credit_cards: true,
    custom_patterns: [],
  });

  const [newPattern, setNewPattern] = useState('');

  const handleTestConnection = async (sinkName: string) => {
    setTestingConnection(sinkName);
    setTestResults(prev => ({ ...prev, [sinkName]: null }));
    
    // Simulate test (in real app, call API)
    await new Promise(resolve => setTimeout(resolve, 1500));
    
    // Simulate random success/failure for demo
    const success = Math.random() > 0.3;
    setTestResults(prev => ({ ...prev, [sinkName]: success ? 'success' : 'error' }));
    setTestingConnection(null);
  };

  const handleSave = async () => {
    setSaving(true);
    // Simulate save (in real app, call API)
    await new Promise(resolve => setTimeout(resolve, 1000));
    setSaving(false);
  };

  const handleAddPattern = () => {
    if (newPattern.trim()) {
      setRedactionConfig(prev => ({
        ...prev,
        custom_patterns: [...prev.custom_patterns, newPattern.trim()],
      }));
      setNewPattern('');
    }
  };

  const handleRemovePattern = (index: number) => {
    setRedactionConfig(prev => ({
      ...prev,
      custom_patterns: prev.custom_patterns.filter((_, i) => i !== index),
    }));
  };

  const tabs: { id: SettingsTab; label: string; icon: typeof CloudArrowUpIcon }[] = [
    { id: 'sinks', label: 'Export Sinks', icon: CloudArrowUpIcon },
    { id: 'redaction', label: 'Privacy & Redaction', icon: ShieldCheckIcon },
    { id: 'general', label: 'General', icon: Cog6ToothIcon },
  ];

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-text-primary">Settings</h2>
          <p className="text-sm text-text-muted mt-1">
            Configure export destinations, privacy settings, and preferences
          </p>
        </div>
        <button
          onClick={handleSave}
          disabled={saving}
          className={clsx('btn btn-primary', saving && 'opacity-50 cursor-not-allowed')}
        >
          {saving ? (
            <>
              <ArrowPathIcon className="w-4 h-4 animate-spin" />
              Saving...
            </>
          ) : (
            'Save Changes'
          )}
        </button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 p-1 bg-bg-secondary rounded-lg border border-border">
        {tabs.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
            className={clsx(
              'flex items-center gap-2 px-4 py-2.5 rounded-md text-sm font-medium transition-all flex-1',
              activeTab === id
                ? 'bg-bg-tertiary text-text-primary'
                : 'text-text-secondary hover:text-text-primary'
            )}
          >
            <Icon className="w-4 h-4" />
            {label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="space-y-6">
        {activeTab === 'sinks' && (
          <>
            {/* JSONL Sink */}
            <SinkCard
              title="JSONL File"
              description="Write events to a local JSON Lines file"
              icon={DocumentTextIcon}
              enabled={jsonlConfig.enabled}
              onToggle={() => setJsonlConfig(prev => ({ ...prev, enabled: !prev.enabled }))}
              testResult={testResults['jsonl']}
              testing={testingConnection === 'jsonl'}
              onTest={() => handleTestConnection('jsonl')}
            >
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-text-secondary mb-2">
                    File Path
                  </label>
                  <input
                    type="text"
                    value={jsonlConfig.path}
                    onChange={(e) => setJsonlConfig(prev => ({ ...prev, path: e.target.value }))}
                    placeholder="/var/log/oisp-sensor/events.jsonl"
                    className="w-full"
                  />
                  <p className="text-xs text-text-muted mt-1">
                    Events will be appended to this file in JSON Lines format
                  </p>
                </div>
              </div>
            </SinkCard>

            {/* OTLP Sink */}
            <SinkCard
              title="OpenTelemetry (OTLP)"
              description="Export to any OTLP-compatible backend"
              icon={CloudArrowUpIcon}
              enabled={otlpConfig.enabled}
              onToggle={() => setOtlpConfig(prev => ({ ...prev, enabled: !prev.enabled }))}
              testResult={testResults['otlp']}
              testing={testingConnection === 'otlp'}
              onTest={() => handleTestConnection('otlp')}
            >
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Endpoint
                    </label>
                    <input
                      type="text"
                      value={otlpConfig.endpoint}
                      onChange={(e) => setOtlpConfig(prev => ({ ...prev, endpoint: e.target.value }))}
                      placeholder="http://localhost:4317"
                      className="w-full"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Protocol
                    </label>
                    <select
                      value={otlpConfig.protocol}
                      onChange={(e) => setOtlpConfig(prev => ({ ...prev, protocol: e.target.value as OtlpConfig['protocol'] }))}
                      className="w-full"
                    >
                      <option value="grpc">gRPC</option>
                      <option value="http/proto">HTTP/Proto</option>
                      <option value="http/json">HTTP/JSON</option>
                    </select>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Toggle
                    enabled={otlpConfig.compression}
                    onToggle={() => setOtlpConfig(prev => ({ ...prev, compression: !prev.compression }))}
                  />
                  <span className="text-sm text-text-secondary">Enable gzip compression</span>
                </div>
                <InfoBox>
                  Compatible with Grafana Cloud, Datadog, Honeycomb, and any OTLP collector.
                  Events are mapped to OpenTelemetry semantic conventions (gen_ai.*, process.*).
                </InfoBox>
              </div>
            </SinkCard>

            {/* Kafka Sink */}
            <SinkCard
              title="Apache Kafka"
              description="Stream events to Kafka topics"
              icon={ServerStackIcon}
              enabled={kafkaConfig.enabled}
              onToggle={() => setKafkaConfig(prev => ({ ...prev, enabled: !prev.enabled }))}
              testResult={testResults['kafka']}
              testing={testingConnection === 'kafka'}
              onTest={() => handleTestConnection('kafka')}
            >
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Bootstrap Servers
                    </label>
                    <input
                      type="text"
                      value={kafkaConfig.brokers}
                      onChange={(e) => setKafkaConfig(prev => ({ ...prev, brokers: e.target.value }))}
                      placeholder="localhost:9092"
                      className="w-full"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Topic
                    </label>
                    <input
                      type="text"
                      value={kafkaConfig.topic}
                      onChange={(e) => setKafkaConfig(prev => ({ ...prev, topic: e.target.value }))}
                      placeholder="oisp-events"
                      className="w-full"
                    />
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      SASL Mechanism
                    </label>
                    <select
                      value={kafkaConfig.sasl_mechanism}
                      onChange={(e) => setKafkaConfig(prev => ({ ...prev, sasl_mechanism: e.target.value as KafkaConfig['sasl_mechanism'] }))}
                      className="w-full"
                    >
                      <option value="none">None</option>
                      <option value="plain">PLAIN</option>
                      <option value="scram-sha-256">SCRAM-SHA-256</option>
                      <option value="scram-sha-512">SCRAM-SHA-512</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Compression
                    </label>
                    <select
                      value={kafkaConfig.compression}
                      onChange={(e) => setKafkaConfig(prev => ({ ...prev, compression: e.target.value as KafkaConfig['compression'] }))}
                      className="w-full"
                    >
                      <option value="none">None</option>
                      <option value="gzip">gzip</option>
                      <option value="snappy">Snappy</option>
                      <option value="lz4">LZ4</option>
                      <option value="zstd">Zstd</option>
                    </select>
                  </div>
                </div>
                {kafkaConfig.sasl_mechanism !== 'none' && (
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="block text-sm font-medium text-text-secondary mb-2">
                        Username
                      </label>
                      <input
                        type="text"
                        value={kafkaConfig.username}
                        onChange={(e) => setKafkaConfig(prev => ({ ...prev, username: e.target.value }))}
                        className="w-full"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-text-secondary mb-2">
                        Password
                      </label>
                      <input
                        type="password"
                        value={kafkaConfig.password}
                        onChange={(e) => setKafkaConfig(prev => ({ ...prev, password: e.target.value }))}
                        className="w-full"
                      />
                    </div>
                  </div>
                )}
              </div>
            </SinkCard>

            {/* Webhook Sink */}
            <SinkCard
              title="Webhook"
              description="POST events to any HTTP endpoint"
              icon={GlobeAltIcon}
              enabled={webhookConfig.enabled}
              onToggle={() => setWebhookConfig(prev => ({ ...prev, enabled: !prev.enabled }))}
              testResult={testResults['webhook']}
              testing={testingConnection === 'webhook'}
              onTest={() => handleTestConnection('webhook')}
            >
              <div className="space-y-4">
                <div className="grid grid-cols-3 gap-4">
                  <div className="col-span-2">
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Endpoint URL
                    </label>
                    <input
                      type="url"
                      value={webhookConfig.url}
                      onChange={(e) => setWebhookConfig(prev => ({ ...prev, url: e.target.value }))}
                      placeholder="https://your-endpoint.com/webhook"
                      className="w-full"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Method
                    </label>
                    <select
                      value={webhookConfig.method}
                      onChange={(e) => setWebhookConfig(prev => ({ ...prev, method: e.target.value as WebhookConfig['method'] }))}
                      className="w-full"
                    >
                      <option value="POST">POST</option>
                      <option value="PUT">PUT</option>
                      <option value="PATCH">PATCH</option>
                    </select>
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Authentication
                    </label>
                    <select
                      value={webhookConfig.auth_type}
                      onChange={(e) => setWebhookConfig(prev => ({ ...prev, auth_type: e.target.value as WebhookConfig['auth_type'] }))}
                      className="w-full"
                    >
                      <option value="none">None</option>
                      <option value="api_key">API Key Header</option>
                      <option value="bearer">Bearer Token</option>
                      <option value="basic">Basic Auth</option>
                    </select>
                  </div>
                  {webhookConfig.auth_type !== 'none' && (
                    <div>
                      <label className="block text-sm font-medium text-text-secondary mb-2">
                        {webhookConfig.auth_type === 'api_key' ? 'API Key' : 
                         webhookConfig.auth_type === 'bearer' ? 'Token' : 'user:password'}
                      </label>
                      <input
                        type="password"
                        value={webhookConfig.auth_value}
                        onChange={(e) => setWebhookConfig(prev => ({ ...prev, auth_value: e.target.value }))}
                        className="w-full"
                      />
                    </div>
                  )}
                </div>
                <div className="flex items-center gap-6">
                  <div className="flex items-center gap-2">
                    <Toggle
                      enabled={webhookConfig.batch_mode}
                      onToggle={() => setWebhookConfig(prev => ({ ...prev, batch_mode: !prev.batch_mode }))}
                    />
                    <span className="text-sm text-text-secondary">Batch mode</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <label className="text-sm text-text-secondary">Max retries:</label>
                    <input
                      type="number"
                      min="0"
                      max="10"
                      value={webhookConfig.max_retries}
                      onChange={(e) => setWebhookConfig(prev => ({ ...prev, max_retries: parseInt(e.target.value) || 0 }))}
                      className="w-16 text-center"
                    />
                  </div>
                </div>
                <InfoBox>
                  Failed events are saved to a dead letter queue file and can be retried later.
                </InfoBox>
              </div>
            </SinkCard>
          </>
        )}

        {activeTab === 'redaction' && (
          <>
            {/* Redaction Mode */}
            <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
              <div className="px-6 py-4 border-b border-border flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-accent-purple/10 flex items-center justify-center">
                  <EyeSlashIcon className="w-5 h-5 text-accent-purple" />
                </div>
                <div>
                  <h3 className="text-sm font-semibold text-text-primary">Redaction Mode</h3>
                  <p className="text-xs text-text-muted">Control how sensitive data is handled</p>
                </div>
              </div>
              <div className="p-6 space-y-4">
                <div className="grid grid-cols-3 gap-4">
                  {(['minimal', 'safe', 'full'] as const).map((mode) => (
                    <button
                      key={mode}
                      onClick={() => setRedactionConfig(prev => ({ ...prev, mode }))}
                      className={clsx(
                        'p-4 rounded-lg border text-left transition-all',
                        redactionConfig.mode === mode
                          ? 'border-accent-purple bg-accent-purple/5'
                          : 'border-border hover:border-text-muted'
                      )}
                    >
                      <h4 className={clsx(
                        'text-sm font-semibold capitalize mb-1',
                        redactionConfig.mode === mode ? 'text-accent-purple' : 'text-text-primary'
                      )}>
                        {mode}
                      </h4>
                      <p className="text-xs text-text-muted">
                        {mode === 'minimal' && 'Only redact explicit secrets like API keys'}
                        {mode === 'safe' && 'Redact PII, secrets, and sensitive patterns'}
                        {mode === 'full' && 'Aggressive redaction of all potential sensitive data'}
                      </p>
                    </button>
                  ))}
                </div>
              </div>
            </div>

            {/* Entity Types */}
            <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
              <div className="px-6 py-4 border-b border-border">
                <h3 className="text-sm font-semibold text-text-primary">Entity Types to Redact</h3>
                <p className="text-xs text-text-muted mt-0.5">Toggle which types of sensitive data to automatically redact</p>
              </div>
              <div className="p-6 space-y-4">
                <div className="flex items-center justify-between p-4 bg-bg-tertiary rounded-lg">
                  <div>
                    <p className="text-sm font-medium text-text-primary">API Keys & Secrets</p>
                    <p className="text-xs text-text-muted">Patterns like sk_*, api_*, secret_*</p>
                  </div>
                  <Toggle
                    enabled={redactionConfig.redact_api_keys}
                    onToggle={() => setRedactionConfig(prev => ({ ...prev, redact_api_keys: !prev.redact_api_keys }))}
                  />
                </div>
                <div className="flex items-center justify-between p-4 bg-bg-tertiary rounded-lg">
                  <div>
                    <p className="text-sm font-medium text-text-primary">Email Addresses</p>
                    <p className="text-xs text-text-muted">Any valid email address format</p>
                  </div>
                  <Toggle
                    enabled={redactionConfig.redact_emails}
                    onToggle={() => setRedactionConfig(prev => ({ ...prev, redact_emails: !prev.redact_emails }))}
                  />
                </div>
                <div className="flex items-center justify-between p-4 bg-bg-tertiary rounded-lg">
                  <div>
                    <p className="text-sm font-medium text-text-primary">Credit Card Numbers</p>
                    <p className="text-xs text-text-muted">16-digit card numbers with Luhn validation</p>
                  </div>
                  <Toggle
                    enabled={redactionConfig.redact_credit_cards}
                    onToggle={() => setRedactionConfig(prev => ({ ...prev, redact_credit_cards: !prev.redact_credit_cards }))}
                  />
                </div>
              </div>
            </div>

            {/* Custom Patterns */}
            <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
              <div className="px-6 py-4 border-b border-border">
                <h3 className="text-sm font-semibold text-text-primary">Custom Redaction Patterns</h3>
                <p className="text-xs text-text-muted mt-0.5">Add regex patterns to match and redact</p>
              </div>
              <div className="p-6 space-y-4">
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={newPattern}
                    onChange={(e) => setNewPattern(e.target.value)}
                    placeholder="Enter regex pattern (e.g., SSN-\d{3}-\d{2}-\d{4})"
                    className="flex-1"
                  />
                  <button
                    onClick={handleAddPattern}
                    className="btn btn-secondary"
                  >
                    Add Pattern
                  </button>
                </div>
                {redactionConfig.custom_patterns.length > 0 && (
                  <div className="space-y-2">
                    {redactionConfig.custom_patterns.map((pattern, index) => (
                      <div
                        key={index}
                        className="flex items-center justify-between p-3 bg-bg-tertiary rounded-lg"
                      >
                        <code className="text-sm font-mono text-text-secondary">{pattern}</code>
                        <button
                          onClick={() => handleRemovePattern(index)}
                          className="text-accent-red hover:text-red-400 text-sm"
                        >
                          Remove
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </>
        )}

        {activeTab === 'general' && (
          <>
            <div className="bg-bg-secondary rounded-xl border border-border overflow-hidden">
              <div className="px-6 py-4 border-b border-border">
                <h3 className="text-sm font-semibold text-text-primary">General Settings</h3>
                <p className="text-xs text-text-muted mt-0.5">Basic sensor configuration</p>
              </div>
              <div className="p-6 space-y-6">
                <div className="grid grid-cols-2 gap-6">
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Log Level
                    </label>
                    <select className="w-full">
                      <option value="error">Error</option>
                      <option value="warn">Warning</option>
                      <option value="info">Info</option>
                      <option value="debug">Debug</option>
                      <option value="trace">Trace</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-text-secondary mb-2">
                      Max Events in Memory
                    </label>
                    <input
                      type="number"
                      defaultValue={1000}
                      min={100}
                      max={10000}
                      className="w-full"
                    />
                  </div>
                </div>
                
                <div className="flex items-center justify-between p-4 bg-bg-tertiary rounded-lg">
                  <div>
                    <p className="text-sm font-medium text-text-primary">Enable WebSocket Broadcast</p>
                    <p className="text-xs text-text-muted">Stream events to connected UI clients in real-time</p>
                  </div>
                  <Toggle enabled={true} onToggle={() => {}} />
                </div>

                <InfoBox>
                  Changes to general settings may require restarting the sensor to take effect.
                </InfoBox>
              </div>
            </div>

            {/* Version Info */}
            <div className="bg-bg-secondary rounded-xl border border-border p-6">
              <h3 className="text-sm font-semibold text-text-primary mb-4">About OISP Sensor</h3>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-text-muted">Version</span>
                  <p className="text-text-primary font-mono">0.1.0</p>
                </div>
                <div>
                  <span className="text-text-muted">Build</span>
                  <p className="text-text-primary font-mono">dev</p>
                </div>
                <div>
                  <span className="text-text-muted">Platform</span>
                  <p className="text-text-primary">Linux x86_64</p>
                </div>
                <div>
                  <span className="text-text-muted">eBPF Status</span>
                  <p className="text-accent-green">Active</p>
                </div>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

// Sink Card Component
function SinkCard({
  title,
  description,
  icon: Icon,
  enabled,
  onToggle,
  testResult,
  testing,
  onTest,
  children,
}: {
  title: string;
  description: string;
  icon: typeof CloudArrowUpIcon;
  enabled: boolean;
  onToggle: () => void;
  testResult: 'success' | 'error' | null;
  testing: boolean;
  onTest: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className={clsx(
      'bg-bg-secondary rounded-xl border overflow-hidden transition-all',
      enabled ? 'border-accent-blue/50' : 'border-border'
    )}>
      <div className="px-6 py-4 border-b border-border flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className={clsx(
            'w-10 h-10 rounded-xl flex items-center justify-center',
            enabled ? 'bg-accent-blue/10' : 'bg-bg-tertiary'
          )}>
            <Icon className={clsx('w-5 h-5', enabled ? 'text-accent-blue' : 'text-text-muted')} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-text-primary">{title}</h3>
            <p className="text-xs text-text-muted">{description}</p>
          </div>
        </div>
        <div className="flex items-center gap-4">
          {/* Test Connection Button */}
          <button
            onClick={onTest}
            disabled={!enabled || testing}
            className={clsx(
              'btn btn-secondary text-xs',
              (!enabled || testing) && 'opacity-50 cursor-not-allowed'
            )}
          >
            {testing ? (
              <>
                <ArrowPathIcon className="w-3 h-3 animate-spin" />
                Testing...
              </>
            ) : testResult === 'success' ? (
              <>
                <CheckCircleIcon className="w-3 h-3 text-accent-green" />
                Connected
              </>
            ) : testResult === 'error' ? (
              <>
                <XCircleIcon className="w-3 h-3 text-accent-red" />
                Failed
              </>
            ) : (
              'Test Connection'
            )}
          </button>
          <Toggle enabled={enabled} onToggle={onToggle} />
        </div>
      </div>
      
      {enabled && (
        <div className="p-6">
          {children}
        </div>
      )}
    </div>
  );
}

// Toggle Component
function Toggle({ enabled, onToggle }: { enabled: boolean; onToggle: () => void }) {
  return (
    <button
      onClick={onToggle}
      className={clsx(
        'toggle',
        enabled ? 'toggle-enabled' : 'toggle-disabled'
      )}
    >
      <span
        className={clsx(
          'toggle-knob',
          enabled ? 'translate-x-5' : 'translate-x-1'
        )}
      />
    </button>
  );
}

// Info Box Component
function InfoBox({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex gap-3 p-3 bg-accent-blue/5 border border-accent-blue/20 rounded-lg">
      <InformationCircleIcon className="w-5 h-5 text-accent-blue flex-shrink-0 mt-0.5" />
      <p className="text-xs text-text-secondary">{children}</p>
    </div>
  );
}

