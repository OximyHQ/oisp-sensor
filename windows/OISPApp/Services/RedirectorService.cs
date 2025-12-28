using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Security.Principal;

namespace OISPApp.Services
{
    /// <summary>
    /// Manages the OISP Redirector process (runs with elevation)
    /// </summary>
    public class RedirectorService
    {
        private Process? _redirectorProcess;

        /// <summary>
        /// Start the redirector process with elevation
        /// </summary>
        public void Start(Settings settings)
        {
            if (_redirectorProcess != null && !_redirectorProcess.HasExited)
            {
                return; // Already running
            }

            // Find redirector executable
            var redirectorPath = FindExecutable(settings.RedirectorPath);
            if (string.IsNullOrEmpty(redirectorPath))
            {
                throw new FileNotFoundException("Could not find oisp-redirector.exe");
            }

            // Build arguments
            var args = "";

            if (settings.EnableTlsMitm)
            {
                args += " --tls-mitm";
            }

            if (!settings.AiEndpointFilterEnabled)
            {
                args += " --all-traffic";
            }

            if (settings.VerboseLogging)
            {
                args += " --verbose";
            }

            if (settings.ProxyPort != 8443)
            {
                args += $" --port {settings.ProxyPort}";
            }

            if (!string.IsNullOrWhiteSpace(settings.ProcessFilter))
            {
                // TODO: Add process filter flag when implemented
            }

            args = args.TrimStart();

            // Start with elevation (UAC prompt)
            var startInfo = new ProcessStartInfo
            {
                FileName = redirectorPath,
                Arguments = args,
                UseShellExecute = true, // Required for elevation
                Verb = "runas",         // Request elevation
                WorkingDirectory = Path.GetDirectoryName(redirectorPath)
            };

            try
            {
                _redirectorProcess = Process.Start(startInfo);
            }
            catch (Win32Exception ex) when (ex.NativeErrorCode == 1223)
            {
                // User cancelled UAC prompt
                throw new OperationCanceledException("Administrator privileges are required. Please accept the UAC prompt.");
            }
        }

        /// <summary>
        /// Stop the redirector process
        /// </summary>
        public void Stop()
        {
            if (_redirectorProcess == null) return;

            try
            {
                if (!_redirectorProcess.HasExited)
                {
                    // For elevated processes, we might need to use taskkill
                    // or signal through a named event
                    try
                    {
                        _redirectorProcess.Kill();
                        _redirectorProcess.WaitForExit(5000);
                    }
                    catch
                    {
                        // If direct kill fails (due to elevation), try taskkill
                        KillProcessAsAdmin(_redirectorProcess.Id);
                    }
                }
            }
            catch
            {
                // Ignore errors during shutdown
            }
            finally
            {
                _redirectorProcess.Dispose();
                _redirectorProcess = null;
            }
        }

        /// <summary>
        /// Check if redirector is running
        /// </summary>
        public bool IsRunning => _redirectorProcess != null && !_redirectorProcess.HasExited;

        /// <summary>
        /// Check if current process is elevated
        /// </summary>
        public static bool IsElevated
        {
            get
            {
                using var identity = WindowsIdentity.GetCurrent();
                var principal = new WindowsPrincipal(identity);
                return principal.IsInRole(WindowsBuiltInRole.Administrator);
            }
        }

        /// <summary>
        /// Kill a process using taskkill (works for elevated processes)
        /// </summary>
        private void KillProcessAsAdmin(int processId)
        {
            try
            {
                var startInfo = new ProcessStartInfo
                {
                    FileName = "taskkill",
                    Arguments = $"/F /PID {processId}",
                    UseShellExecute = true,
                    Verb = "runas",
                    CreateNoWindow = true
                };
                Process.Start(startInfo)?.WaitForExit(5000);
            }
            catch
            {
                // Ignore errors
            }
        }

        /// <summary>
        /// Find executable path, checking multiple locations
        /// </summary>
        private string? FindExecutable(string exeName)
        {
            // Try the configured path directly
            if (File.Exists(exeName))
            {
                return Path.GetFullPath(exeName);
            }

            // Try in same directory as OISPApp
            var appDir = AppDomain.CurrentDomain.BaseDirectory;
            var inAppDir = Path.Combine(appDir, exeName);
            if (File.Exists(inAppDir))
            {
                return inAppDir;
            }

            // Try in parent directory (for development)
            var parentDir = Path.GetDirectoryName(appDir);
            if (parentDir != null)
            {
                var inParent = Path.Combine(parentDir, exeName);
                if (File.Exists(inParent))
                {
                    return inParent;
                }
            }

            // Try in release/debug build directories
            var possiblePaths = new[]
            {
                Path.Combine(appDir, "..", "..", "target", "release", exeName),
                Path.Combine(appDir, "..", "..", "target", "debug", exeName),
                Path.Combine(appDir, "bin", exeName),
            };

            foreach (var path in possiblePaths)
            {
                if (File.Exists(path))
                {
                    return Path.GetFullPath(path);
                }
            }

            // Try PATH
            var pathEnv = Environment.GetEnvironmentVariable("PATH");
            if (!string.IsNullOrEmpty(pathEnv))
            {
                foreach (var dir in pathEnv.Split(Path.PathSeparator))
                {
                    var fullPath = Path.Combine(dir, exeName);
                    if (File.Exists(fullPath))
                    {
                        return fullPath;
                    }
                }
            }

            return null;
        }
    }
}
