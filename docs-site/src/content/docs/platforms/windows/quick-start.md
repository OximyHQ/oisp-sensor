---
title: Windows Quick Start
description: Get started with OISP Sensor on Windows in 5 minutes
---

Get started with full SSL/TLS capture on Windows in 5 minutes.

## Using the System Tray App

### 1. Launch OISP

Double-click `OISPApp.exe` or launch from the Start Menu.

### 2. Install CA Certificate

Right-click the tray icon → **"Install CA Certificate"**

This adds the OISP CA to your trusted certificates for HTTPS interception.

### 3. Start Capture

Right-click the tray icon → **"Start Capture"**

Accept the UAC prompt (Administrator required for packet capture).

### 4. Generate AI Activity

Use any AI tool - Python, Node.js, or any application:

```powershell
# Python + OpenAI
pip install openai
python -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'Hello'}]).choices[0].message.content)"
```

### 5. View Events

Right-click the tray icon → **"View Logs"**

Or check the events file at `%USERPROFILE%\Documents\OISP\events.jsonl`

## Using Command Line

For more control, use the command line directly:

```powershell
# Terminal 1: Start the sensor (normal user)
.\oisp-sensor.exe record --output events.jsonl

# Terminal 2 (Run as Administrator): Start the redirector
.\oisp-redirector.exe --tls-mitm

# Terminal 3: Make AI API calls
python -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'Hello'}]))"

# Events will appear in events.jsonl
```

## Command Line Options

### oisp-sensor options

```powershell
oisp-sensor.exe record --output events.jsonl    # Record to file
oisp-sensor.exe record --web                    # With web dashboard
oisp-sensor.exe record --verbose                # Verbose logging
```

### oisp-redirector options

```powershell
oisp-redirector.exe --tls-mitm                  # Enable HTTPS interception
oisp-redirector.exe --all-traffic               # Capture all traffic (not just AI)
oisp-redirector.exe --port 8443                 # Custom proxy port
oisp-redirector.exe --verbose                   # Verbose logging
oisp-redirector.exe --help                      # Show all options
```

## Web Dashboard

Start with the web dashboard for a visual interface:

```powershell
# Via tray app: Right-click → "Start with Web UI"

# Or via command line:
oisp-sensor.exe record --web --output events.jsonl
```

Then open http://localhost:7777 in your browser.

## Example Output

```json
{
  "event_type": "ai.request",
  "timestamp": "2024-12-28T12:00:00Z",
  "provider": "openai",
  "model": "gpt-4o-mini",
  "process": {
    "pid": 1234,
    "name": "python.exe",
    "path": "C:\\Python312\\python.exe"
  },
  "data": {
    "messages": [
      {"role": "user", "content": "Hello"}
    ]
  }
}
```

## Troubleshooting

### "Could not start capture"

1. Ensure you accepted the UAC prompt
2. Check if another application is using WinDivert
3. Temporarily disable antivirus and try again

### "HTTPS traffic not captured"

1. Ensure CA certificate is installed: Right-click tray → "Install CA Certificate"
2. Verify CA is in Trusted Root store: `certmgr.msc` → Trusted Root Certification Authorities
3. Some applications may use certificate pinning (cannot be intercepted)

### Check Logs

```powershell
# View redirector logs
.\oisp-redirector.exe --verbose

# View sensor logs
.\oisp-sensor.exe record --verbose --output events.jsonl

# Check captured events
Get-Content events.jsonl | Select-Object -Last 10
```

## Next Steps

- [Overview](./overview) - Full architecture details
- [Installation](./installation) - Detailed installation guide
