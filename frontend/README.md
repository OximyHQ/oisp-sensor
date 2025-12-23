# OISP Sensor Frontend

A modern React/Next.js frontend for OISP Sensor, providing an AgentSight-style UI for AI agent observability.

## Features

- **Process Tree View**: Hierarchical visualization of processes with their AI prompts, API calls, and file operations
- **Timeline View**: Chronological event stream with visual timeline
- **Log View**: Raw event log with search and filtering
- **Real-time Updates**: WebSocket connection for live event streaming
- **Dark Theme**: GitHub-inspired dark theme with syntax highlighting

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

The dev server runs on `http://localhost:3000` and proxies API requests to the backend at `http://localhost:7777`.

## Building for Production

```bash
# Build static export
npm run build
```

This generates a static export in the `out/` directory that gets embedded into the Rust binary via `rust-embed`.

## Architecture

### Event Flow

```
Backend (Rust)                    Frontend (React)
     │                                  │
     ├──[/api/web-events]──────────────►│ Initial load
     │                                  │
     ├──[/ws WebSocket]────────────────►│ Real-time updates
     │                                  │
     └──[WebEvent format]──────────────►│ Simplified, PID-centric
```

### Key Types

- `WebEvent`: Simplified event format with PID as primary key
- `ProcessNode`: Tree node for process hierarchy
- `ParsedEvent`: Display-friendly event with title/subtitle

### Components

```
src/
├── app/
│   ├── layout.tsx      # Root layout
│   ├── page.tsx        # Main page with view switching
│   └── globals.css     # Tailwind + custom styles
├── components/
│   ├── Header.tsx      # Stats and connection status
│   ├── Navigation.tsx  # View tabs
│   ├── ProcessTreeView.tsx  # Main tree view
│   ├── ProcessNode.tsx # Recursive process node
│   ├── EventBlock.tsx  # Event display
│   ├── TimelineView.tsx
│   ├── LogView.tsx
│   └── EmptyState.tsx
├── lib/
│   └── useEvents.ts    # API + WebSocket hook
├── types/
│   └── event.ts        # TypeScript types
└── utils/
    └── eventParsers.ts # Event parsing + tree building
```

## Integration with Backend

The frontend is embedded into the Rust binary using `rust-embed`. When building the sensor:

1. Run `npm run build` in the frontend directory
2. The `out/` directory is embedded via `rust-embed`
3. The Rust server serves the frontend at `/`

Legacy pages remain available at `/legacy` and `/legacy/timeline`.

