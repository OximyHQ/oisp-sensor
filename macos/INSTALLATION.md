# OISP macOS Installation Guide

This guide walks you through installing and setting up OISP on macOS.

## Prerequisites

- macOS 13.0 (Ventura) or later
- Administrator access
- 50 MB free disk space

## Installation Methods

### Method 1: DMG Installer (Recommended)

1. **Download the DMG**
   - Get the latest `OISP-x.x.x.dmg` from the releases page

2. **Install the Application**
   - Open the DMG file
   - Drag `OISP.app` to the Applications folder
   - Eject the DMG

3. **First Launch**
   - Open OISP from Applications (or Spotlight)
   - You may see "OISP is from an identified developer" - click **Open**
   - The setup wizard will guide you through the remaining steps

### Method 2: Build from Source

See the build instructions in [README.md](README.md#building-from-source).

## Setup Wizard

When you first launch OISP, a setup wizard guides you through:

### Step 1: Enable Network Extension

OISP requires a Network Extension to capture AI API traffic.

1. Click **Enable Extension**
2. System Settings will open to **Privacy & Security**
3. Find "System Extension Blocked" and click **Allow**
4. Enter your password when prompted

**Note:** You may need to restart your Mac for the extension to load.

### Step 2: Trust CA Certificate

OISP uses a local Certificate Authority to decrypt HTTPS traffic.

1. Click **Trust Certificate**
2. Enter your password when prompted
3. The OISP CA will be added to your keychain as trusted

**Manual Trust (if automatic fails):**
```bash
# Export the certificate
/Applications/OISP.app/Contents/Resources/export-ca.sh

# Open Keychain Access
open -a "Keychain Access"

# Drag the exported OISP-CA.pem to "System" keychain
# Double-click the certificate
# Expand "Trust" and set "When using this certificate" to "Always Trust"
```

### Step 3: Complete

OISP is now ready to capture AI API traffic!

## Verification

### Check Extension Status

```bash
# List system extensions
systemextensionsctl list

# Expected output (similar to):
# 1 extension(s)
# --- com.apple.system_extension.network_extension
# enabled	active	teamID	bundleID (version)	name	[state]
# *	*	TEAMID	com.oisp.networkextension (1.0/1)	OISP Network Extension	[activated enabled]
```

### Check CA Trust

```bash
# List trusted certificates
security find-certificate -a -c "OISP" /Library/Keychains/System.keychain

# Should show the OISP CA certificate
```

### Test Capture

1. Make an API call to OpenAI or another provider:
   ```bash
   curl https://api.openai.com/v1/models -H "Authorization: Bearer sk-test"
   ```

2. Check the OISP menu bar icon - it should show the request was captured

3. Open the dashboard (click Dashboard in the menu) to see captured requests

## Updating OISP

1. Download the new DMG
2. Quit the running OISP app
3. Replace the app in Applications
4. Relaunch OISP

The Network Extension will be updated automatically.

## Uninstalling OISP

### Complete Uninstall

1. **Quit OISP**
   - Click the menu bar icon → Quit

2. **Remove the Extension**
   ```bash
   sudo systemextensionsctl uninstall com.oisp.networkextension
   ```

3. **Remove the CA Certificate**
   - Open Keychain Access
   - Search for "OISP"
   - Delete the OISP CA certificate

4. **Delete the Application**
   ```bash
   rm -rf /Applications/OISP.app
   ```

5. **Remove Configuration**
   ```bash
   rm -rf ~/Library/Application\ Support/OISP
   rm -rf ~/.config/oisp
   ```

## Enterprise Deployment

For deploying OISP across multiple machines:

### MDM Configuration Profile

Create a configuration profile to:
1. Pre-approve the system extension
2. Pre-trust the CA certificate

Example MDM payload:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>PayloadContent</key>
    <array>
        <dict>
            <key>PayloadType</key>
            <string>com.apple.system-extension-policy</string>
            <key>AllowedSystemExtensions</key>
            <dict>
                <key>TEAMID</key>
                <array>
                    <string>com.oisp.networkextension</string>
                </array>
            </dict>
            <key>AllowedSystemExtensionTypes</key>
            <dict>
                <key>TEAMID</key>
                <array>
                    <string>NetworkExtension</string>
                </array>
            </dict>
        </dict>
    </array>
    <key>PayloadDisplayName</key>
    <string>OISP System Extension</string>
    <key>PayloadIdentifier</key>
    <string>com.oisp.sysext</string>
    <key>PayloadType</key>
    <string>Configuration</string>
    <key>PayloadVersion</key>
    <integer>1</integer>
</dict>
</plist>
```

### Silent Installation

```bash
# Copy app
cp -R /path/to/OISP.app /Applications/

# Approve extension (requires MDM profile)
# Extension will auto-install on first launch

# Trust CA (requires MDM or manual approval)
security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain /path/to/OISP-CA.pem
```
