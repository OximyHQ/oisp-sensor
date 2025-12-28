using System;
using System.Diagnostics;
using System.IO;

namespace OISPApp.Services
{
    /// <summary>
    /// Manages the OISP Sensor process
    /// </summary>
    public class SensorService
    {
        private Process? _sensorProcess;

        /// <summary>
        /// Start the sensor process
        /// </summary>
        public void Start(Settings settings)
        {
            if (_sensorProcess != null && !_sensorProcess.HasExited)
            {
                return; // Already running
            }

            // Ensure output directory exists
            var outputDir = Path.GetDirectoryName(settings.OutputPath);
            if (outputDir != null && !Directory.Exists(outputDir))
            {
                Directory.CreateDirectory(outputDir);
            }

            // Find sensor executable
            var sensorPath = FindExecutable(settings.SensorPath);
            if (string.IsNullOrEmpty(sensorPath))
            {
                throw new FileNotFoundException("Could not find oisp-sensor.exe");
            }

            // Build arguments
            var args = $"record --output \"{settings.OutputPath}\"";
            if (settings.VerboseLogging)
            {
                args += " --verbose";
            }

            var startInfo = new ProcessStartInfo
            {
                FileName = sensorPath,
                Arguments = args,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                WorkingDirectory = Path.GetDirectoryName(sensorPath)
            };

            _sensorProcess = new Process { StartInfo = startInfo };

            // Log output if verbose
            if (settings.VerboseLogging)
            {
                _sensorProcess.OutputDataReceived += (s, e) =>
                {
                    if (e.Data != null) Debug.WriteLine($"[Sensor] {e.Data}");
                };
                _sensorProcess.ErrorDataReceived += (s, e) =>
                {
                    if (e.Data != null) Debug.WriteLine($"[Sensor ERR] {e.Data}");
                };
            }

            _sensorProcess.Start();

            if (settings.VerboseLogging)
            {
                _sensorProcess.BeginOutputReadLine();
                _sensorProcess.BeginErrorReadLine();
            }
        }

        /// <summary>
        /// Stop the sensor process
        /// </summary>
        public void Stop()
        {
            if (_sensorProcess == null) return;

            try
            {
                if (!_sensorProcess.HasExited)
                {
                    // Send Ctrl+C equivalent (graceful shutdown)
                    // On Windows, we need to use GenerateConsoleCtrlEvent or just kill
                    _sensorProcess.Kill();
                    _sensorProcess.WaitForExit(5000);
                }
            }
            catch
            {
                // Ignore errors during shutdown
            }
            finally
            {
                _sensorProcess.Dispose();
                _sensorProcess = null;
            }
        }

        /// <summary>
        /// Check if sensor is running
        /// </summary>
        public bool IsRunning => _sensorProcess != null && !_sensorProcess.HasExited;

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
