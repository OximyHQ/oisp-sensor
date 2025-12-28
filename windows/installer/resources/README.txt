OISP Sensor for Windows
=======================

Version: 0.1.0

OISP Sensor captures and logs AI API traffic on your system, providing
visibility into how AI models are being used by your applications.

FEATURES
--------

- Captures traffic to OpenAI, Anthropic, Google AI, and 40+ other providers
- TLS/HTTPS interception for complete visibility
- Process attribution - see which application made each request
- Low-overhead system-level capture
- JSON Lines output for easy analysis

QUICK START
-----------

1. Launch "OISP Sensor" from Start Menu or desktop shortcut

2. Right-click the system tray icon and click "Install CA Certificate"
   - Accept the security prompt to trust HTTPS interception
   - This is required for capturing HTTPS traffic

3. Right-click the tray icon and click "Start Capture"
   - Accept the UAC prompt (Admin required for packet capture)

4. Run your AI applications as usual

5. View captured events:
   - Right-click tray icon -> "View Logs"
   - Or open: Documents\OISP\events.jsonl

SYSTEM REQUIREMENTS
-------------------

- Windows 10/11 (64-bit)
- Administrator privileges for capture
- ~50 MB disk space

COMPONENTS
----------

- OISPApp.exe       System tray application (user interface)
- oisp-sensor.exe   Event processing and export
- oisp-redirector.exe   Packet capture (requires elevation)
- WinDivert.dll     Packet capture library
- WinDivert64.sys   Packet capture driver (kernel mode)

DATA STORAGE
------------

User data is stored at:
  %LOCALAPPDATA%\OISP\

This includes:
  - settings.json    Application settings
  - oisp-ca.crt      CA certificate for HTTPS interception
  - oisp-ca.key      CA private key (keep secure!)

Captured events are stored at:
  %USERPROFILE%\Documents\OISP\events.jsonl

TROUBLESHOOTING
---------------

"Could not start capture"
  - Make sure you accepted the UAC prompt
  - Check if antivirus is blocking WinDivert

"HTTPS traffic not captured"
  - Install the CA certificate from the tray menu
  - Some applications may pin certificates (can't intercept)

"High CPU usage"
  - Enable AI endpoint filtering (captures only AI traffic)
  - Available in Settings

UNINSTALLATION
--------------

1. Use Windows "Add or Remove Programs"
2. Or run: "C:\Program Files\OISP Sensor\uninstall.exe"

Note: User data is NOT removed during uninstallation.
To fully remove data, delete: %LOCALAPPDATA%\OISP\

SUPPORT
-------

GitHub: https://github.com/your-org/oisp-sensor
Issues: https://github.com/your-org/oisp-sensor/issues

LICENSE
-------

MIT License - See LICENSE.txt for details

WinDivert is licensed under LGPL-3.0
