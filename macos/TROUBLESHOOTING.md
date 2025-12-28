# OISP macOS Troubleshooting Guide

This guide covers common issues and solutions for OISP on macOS.

## Quick Diagnostics

Run the diagnostics script:
```bash
/Applications/OISP.app/Contents/Resources/diagnose.sh
```

Or check manually:

```bash
# Check extension status
systemextensionsctl list | grep oisp

# Check if extension process is running
ps aux | grep -i oisp

# Check recent logs
log show --predicate 'subsystem == "com.oisp"' --last 5m

# Check socket
ls -la /tmp/oisp.sock
```

## Common Issues

### Extension Not Loading

**Symptoms:**
- OISP shows "Extension not enabled"
- No traffic being captured

**Solutions:**

1. **Check System Settings**
   - Open System Settings → Privacy & Security
   - Scroll down to "Security"
   - If you see "System Extension Blocked", click Allow

2. **Restart Required**
   - Some extension changes require a restart
   - Reboot your Mac and try again

3. **Re-register Extension**
   ```bash
   # Remove existing extension
   sudo systemextensionsctl uninstall com.oisp.networkextension

   # Quit and relaunch OISP
   killall OISP
   open /Applications/OISP.app
   ```

4. **Check SIP Status**
   - System Integrity Protection must be enabled
   ```bash
   csrutil status
   # Should show: System Integrity Protection status: enabled.
   ```

### Certificate Trust Issues

**Symptoms:**
- OISP shows "CA not trusted"
- SSL errors when making API calls
- Apps refuse to connect to AI APIs

**Solutions:**

1. **Manual Trust**
   ```bash
   # Export the CA certificate
   openssl x509 -in ~/Library/Application\ Support/OISP/ca.pem -out /tmp/oisp-ca.crt

   # Open Keychain Access
   open -a "Keychain Access"

   # Drag /tmp/oisp-ca.crt to System keychain
   # Double-click the certificate
   # Expand Trust → When using this certificate: Always Trust
   ```

2. **Check Trust Settings**
   ```bash
   security find-certificate -a -c "OISP" -p | head -20
   security trust-settings-export -d /tmp/trust.plist
   plutil -p /tmp/trust.plist | grep -A5 OISP
   ```

3. **Regenerate Certificate**
   ```bash
   # Remove existing CA
   rm -rf ~/Library/Application\ Support/OISP/ca.pem
   rm -rf ~/Library/Application\ Support/OISP/ca.key

   # Restart OISP to regenerate
   killall OISP
   open /Applications/OISP.app
   ```

### No Traffic Being Captured

**Symptoms:**
- Extension is enabled
- CA is trusted
- But no API calls appear in dashboard

**Solutions:**

1. **Check Target Endpoints**
   - OISP only captures traffic to known AI endpoints
   - Custom endpoints must be added to configuration

2. **Check Socket Connection**
   ```bash
   # Verify socket exists
   ls -la /tmp/oisp.sock

   # Check if oisp-sensor is running
   ps aux | grep oisp-sensor
   ```

3. **Test with curl**
   ```bash
   # This should be captured
   curl -v https://api.openai.com/v1/models 2>&1 | grep -i "issuer"

   # Should show OISP CA as issuer
   ```

4. **Check Extension Logs**
   ```bash
   log stream --predicate 'subsystem == "com.oisp.networkextension"' --level debug
   ```

### Performance Issues

**Symptoms:**
- Slow API responses
- High CPU usage
- Memory pressure

**Solutions:**

1. **Reduce Log Level**
   - Open OISP Settings → Advanced
   - Set Log Level to "Warning" or "Error"

2. **Increase Buffer Size**
   ```bash
   # Edit config
   nano ~/.config/oisp/config.toml

   # Add or modify:
   [macos]
   buffer_size = 10000
   ```

3. **Exclude Non-AI Traffic**
   - OISP should only intercept AI endpoints
   - Check that custom endpoints are correct

### App Crashes

**Symptoms:**
- OISP quits unexpectedly
- Extension crashes

**Solutions:**

1. **Check Crash Reports**
   ```bash
   # Open Console app
   open -a Console

   # Look in User Reports for OISP crashes
   ```

2. **Reset State**
   ```bash
   # Remove all state
   rm -rf ~/Library/Application\ Support/OISP
   rm -rf ~/Library/Preferences/com.oisp.*

   # Relaunch
   open /Applications/OISP.app
   ```

3. **Check for Conflicts**
   - Other VPN or proxy software may conflict
   - Try disabling other network extensions temporarily

## Log Collection

For bug reports, collect these logs:

```bash
# Create diagnostic bundle
mkdir -p /tmp/oisp-diag
cd /tmp/oisp-diag

# System extension status
systemextensionsctl list > extensions.txt

# Recent logs
log show --predicate 'subsystem CONTAINS "oisp"' --last 1h > logs.txt

# Configuration (sanitized)
cat ~/.config/oisp/config.toml | grep -v key > config.txt

# Network extension state
networksetup -listallnetworkservices > network.txt

# Create archive
tar -czvf oisp-diagnostics.tar.gz /tmp/oisp-diag
```

## Reset Everything

If all else fails, completely reset OISP:

```bash
#!/bin/bash
# Reset OISP completely

echo "Stopping OISP..."
killall OISP 2>/dev/null

echo "Removing extension..."
sudo systemextensionsctl uninstall com.oisp.networkextension 2>/dev/null

echo "Removing CA from keychain..."
security delete-certificate -c "OISP CA" /Library/Keychains/System.keychain 2>/dev/null

echo "Removing configuration..."
rm -rf ~/Library/Application\ Support/OISP
rm -rf ~/.config/oisp
rm -rf ~/Library/Preferences/com.oisp.*

echo "Removing socket..."
rm -f /tmp/oisp.sock

echo "Done. Please restart your Mac before reinstalling OISP."
```

## Getting Help

If you're still having issues:

1. Check the [GitHub Issues](https://github.com/oisp/oisp-sensor/issues)
2. Search for similar problems
3. Create a new issue with:
   - macOS version
   - OISP version
   - Diagnostic bundle (see above)
   - Steps to reproduce

## Known Issues

### macOS 14 (Sonoma) Specific

- First launch may require two approvals (extension + network filter)
- Some VPN apps may conflict with transparent proxy

### macOS 13 (Ventura) Specific

- Extension approval UI may be hidden behind other windows
- Check System Settings manually if prompted

### Apple Silicon vs Intel

- Both architectures are supported
- If you're on Intel and having issues, ensure you have the x86_64 build
