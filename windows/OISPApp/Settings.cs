using Newtonsoft.Json;
using System;
using System.IO;

namespace OISPApp
{
    /// <summary>
    /// Application settings, persisted to JSON file
    /// </summary>
    public class Settings
    {
        /// <summary>
        /// Path to output events file
        /// </summary>
        public string OutputPath { get; set; } = GetDefaultOutputPath();

        /// <summary>
        /// Auto-start capture when app launches
        /// </summary>
        public bool AutoStartCapture { get; set; } = false;

        /// <summary>
        /// Enable TLS MITM mode (requires CA installation)
        /// </summary>
        public bool EnableTlsMitm { get; set; } = true;

        /// <summary>
        /// Only capture traffic to AI API endpoints
        /// </summary>
        public bool AiEndpointFilterEnabled { get; set; } = true;

        /// <summary>
        /// Process name filter (empty = all processes)
        /// </summary>
        public string ProcessFilter { get; set; } = "";

        /// <summary>
        /// Proxy port for redirected traffic
        /// </summary>
        public int ProxyPort { get; set; } = 8443;

        /// <summary>
        /// Web dashboard port
        /// </summary>
        public int WebPort { get; set; } = 7777;

        /// <summary>
        /// Verbose logging
        /// </summary>
        public bool VerboseLogging { get; set; } = false;

        /// <summary>
        /// Path to sensor executable
        /// </summary>
        public string SensorPath { get; set; } = "oisp-sensor.exe";

        /// <summary>
        /// Path to redirector executable
        /// </summary>
        public string RedirectorPath { get; set; } = "oisp-redirector.exe";

        private static string GetDefaultOutputPath()
        {
            var documents = Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments);
            return Path.Combine(documents, "OISP", "events.jsonl");
        }

        private static string GetSettingsPath()
        {
            var localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            return Path.Combine(localAppData, "OISP", "settings.json");
        }

        /// <summary>
        /// Load settings from disk, or create default settings
        /// </summary>
        public static Settings Load()
        {
            var path = GetSettingsPath();
            try
            {
                if (File.Exists(path))
                {
                    var json = File.ReadAllText(path);
                    return JsonConvert.DeserializeObject<Settings>(json) ?? new Settings();
                }
            }
            catch
            {
                // If loading fails, return defaults
            }
            return new Settings();
        }

        /// <summary>
        /// Save settings to disk
        /// </summary>
        public void Save()
        {
            var path = GetSettingsPath();
            try
            {
                var dir = Path.GetDirectoryName(path);
                if (dir != null && !Directory.Exists(dir))
                {
                    Directory.CreateDirectory(dir);
                }

                var json = JsonConvert.SerializeObject(this, Formatting.Indented);
                File.WriteAllText(path, json);
            }
            catch
            {
                // Ignore save errors
            }
        }
    }
}
