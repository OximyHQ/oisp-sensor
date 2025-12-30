---
title: Python + LangChain Agent
description: Build AI agents with LangChain and monitor tool calls
---


Build an agent with tools using LangChain and monitor all interactions.

## Overview

**What this demonstrates:**
- LangChain agent with tools
- Tool call tracing
- Multi-turn conversations
- Agent reasoning patterns

**Repository:** `oisp-cookbook/python/03-langchain-agent`

## Key Code

```python
from langchain.agents import initialize_agent, Tool
from langchain.llms import OpenAI
from langchain.tools import DuckDuckGoSearchRun

# Define tools
search = DuckDuckGoSearchRun()
tools = [
    Tool(
        name="Search",
        func=search.run,
        description="Search the web for current information"
    )
]

# Create agent
agent = initialize_agent(
    tools=tools,
    llm=OpenAI(temperature=0),
    agent="zero-shot-react-description"
)

# Run agent
result = agent.run("What's the weather in San Francisco?")
```

## Captured Events

**Full agent execution trace:**

1. `ai.request` - User query to agent
2. `ai.response` - Agent decides to use search tool
3. `agent.tool_call` - Search tool invoked
4. `agent.tool_result` - Search results
5. `ai.request` - Agent sends results to LLM
6. `ai.response` - Final answer to user

## Event Example

**Tool call event:**

```json
{
  "event_type": "agent.tool_call",
  "data": {
    "tool_name": "Search",
    "arguments": {
      "query": "San Francisco weather"
    }
  }
}
```

## Running

```bash
cd oisp-cookbook/python/03-langchain-agent
export OPENAI_API_KEY=sk-...
docker-compose up
```

## Use Cases

- Debugging agent behavior
- Understanding tool usage patterns
- Tracking agentic costs
- Optimizing agent prompts
