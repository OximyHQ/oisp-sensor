---
title: macOS Quick Start
description: Get started with OISP Sensor on macOS
---

# macOS Quick Start

## Start the Sensor

```bash
sudo oisp-sensor
```

You'll see metadata events for AI activity (provider detection, timing, but no content).

## Generate AI Activity

Use any AI tool:

```bash
# Python + OpenAI
pip install openai
python3 -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'Hi'}]).choices[0].message.content)"
```

## View Events

The sensor will capture:
- Network connection to api.openai.com
- Process execution (python3)
- Provider detection (OpenAI)

**Note:** Full request/response content is not captured yet on macOS.

## Export to File

```bash
sudo oisp-sensor record --output /tmp/events.jsonl --no-ui
```

## What's Missing

See [Limitations](./limitations) for details on current macOS restrictions.
