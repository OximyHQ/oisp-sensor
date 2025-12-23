/**
 * WebEvent - Simplified event format from the backend
 * 
 * PID is the primary organizing key. Every event has pid and comm.
 */
export interface WebEvent {
  /** Unique event ID */
  id: string;
  /** Unix timestamp in milliseconds */
  timestamp: number;
  /** Event type */
  type: WebEventType;
  /** Process ID - REQUIRED, primary grouping key */
  pid: number;
  /** Parent process ID - for building process tree */
  ppid?: number;
  /** Process name - REQUIRED (e.g., "claude", "python3") */
  comm: string;
  /** Event-specific data */
  data: WebEventData;
}

export type WebEventType =
  | 'ai_prompt'
  | 'ai_response'
  | 'process_exec'
  | 'process_exit'
  | 'file_open'
  | 'file_write'
  | 'network_connect'
  | 'other';

export type WebEventData =
  | AiPromptData
  | AiResponseData
  | ProcessExecData
  | ProcessExitData
  | FileOpData
  | NetworkData
  | Record<string, unknown>;

export interface AiPromptData {
  provider: string;
  model: string;
  message_count: number;
  streaming: boolean;
  tool_count: number;
  estimated_tokens?: number;
  endpoint?: string;
}

export interface AiResponseData {
  provider: string;
  model: string;
  latency_ms?: number;
  input_tokens?: number;
  output_tokens?: number;
  finish_reason?: string;
  tool_calls: number;
  success: boolean;
}

export interface ProcessExecData {
  exe?: string;
  cmdline?: string;
  cwd?: string;
}

export interface ProcessExitData {
  exit_code: number;
  duration_ms?: number;
}

export interface FileOpData {
  path: string;
  operation: string;
  bytes?: number;
}

export interface NetworkData {
  remote_addr: string;
  remote_port: number;
  protocol: string;
}

/**
 * API response for /api/web-events
 */
export interface WebEventsResponse {
  events: WebEvent[];
  total: number;
}

/**
 * ProcessNode - For building the process tree view
 */
export interface ProcessNode {
  /** Process ID */
  pid: number;
  /** Parent process ID */
  ppid?: number;
  /** Process name */
  comm: string;
  /** Child processes */
  children: ProcessNode[];
  /** Events for this process */
  events: ParsedEvent[];
  /** Timeline items (events + child spawns) in chronological order */
  timeline: TimelineItem[];
  /** Whether the node is expanded in the UI */
  isExpanded: boolean;
}

/**
 * Timeline item - either an event or a child process spawn
 */
export interface TimelineItem {
  type: 'event' | 'process';
  timestamp: number;
  event?: ParsedEvent;
  process?: ProcessNode;
}

/**
 * Parsed event with display-friendly data
 */
export interface ParsedEvent {
  id: string;
  timestamp: number;
  type: WebEventType;
  /** Display title */
  title: string;
  /** Display subtitle */
  subtitle?: string;
  /** Original event data */
  data: WebEventData;
  /** Whether expanded in UI */
  isExpanded: boolean;
}

/**
 * Type guards for event data
 */
export function isAiPromptData(data: WebEventData): data is AiPromptData {
  return 'provider' in data && 'message_count' in data;
}

export function isAiResponseData(data: WebEventData): data is AiResponseData {
  return 'provider' in data && 'tool_calls' in data;
}

export function isProcessExecData(data: WebEventData): data is ProcessExecData {
  return 'exe' in data || 'cmdline' in data;
}

export function isProcessExitData(data: WebEventData): data is ProcessExitData {
  return 'exit_code' in data;
}

export function isFileOpData(data: WebEventData): data is FileOpData {
  return 'path' in data && 'operation' in data;
}

export function isNetworkData(data: WebEventData): data is NetworkData {
  return 'remote_addr' in data && 'remote_port' in data;
}

