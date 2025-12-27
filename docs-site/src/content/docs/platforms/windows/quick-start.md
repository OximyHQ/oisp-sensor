---
title: Windows Quick Start
description: Get started with OISP Sensor on Windows
---


## Start the Sensor

```powershell
oisp-sensor.exe
```

You'll see metadata events for AI activity (provider detection, timing, but no content).

## Generate AI Activity

Use any AI tool:

```powershell
# Python + OpenAI
pip install openai
python -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'Hi'}]).choices[0].message.content)"
```

## View Events

The sensor will capture:
- Network connection to api.openai.com
- Process execution (python.exe)
- Provider detection (OpenAI)

**Note:** Full request/response content is not captured yet on Windows.

## Export to File

```powershell
oisp-sensor.exe record --output C:\logs\events.jsonl --no-ui
```

## Install as Service (Optional)

For persistent monitoring:

```powershell
# Run as Administrator
oisp-sensor.exe install-service
```

## Next Steps

- [Overview](./overview) - Current capabilities
- **[Linux Guide](/platforms/linux/)** - For full capture
