using Hardcodet.Wpf.TaskbarNotification;
using OISPApp.Services;
using System;
using System.IO;
using System.Windows;
using System.Windows.Input;

namespace OISPApp
{
    /// <summary>
    /// OISP Sensor System Tray Application
    /// </summary>
    public partial class App : Application
    {
        private TaskbarIcon? _trayIcon;
        private SensorService? _sensorService;
        private RedirectorService? _redirectorService;
        private CertificateService? _certificateService;
        private SettingsWindow? _settingsWindow;
        private bool _isCapturing;

        /// <summary>
        /// Command to open settings window
        /// </summary>
        public static RoutedCommand OpenSettingsCommand { get; } = new RoutedCommand();

        private void Application_Startup(object sender, StartupEventArgs e)
        {
            // Initialize services
            _certificateService = new CertificateService();
            _sensorService = new SensorService();
            _redirectorService = new RedirectorService();

            // Get the tray icon from resources
            _trayIcon = (TaskbarIcon)FindResource("TrayIcon");

            // Check if CA is installed
            UpdateTrayStatus();

            // Auto-start if configured
            var settings = Settings.Load();
            if (settings.AutoStartCapture)
            {
                StartCapture();
            }
        }

        private void Application_Exit(object sender, ExitEventArgs e)
        {
            // Stop capture if running
            StopCapture();

            // Dispose tray icon
            _trayIcon?.Dispose();
        }

        private void StartCapture_Click(object sender, RoutedEventArgs e)
        {
            StartCapture();
        }

        private void StopCapture_Click(object sender, RoutedEventArgs e)
        {
            StopCapture();
        }

        private void InstallCA_Click(object sender, RoutedEventArgs e)
        {
            try
            {
                if (_certificateService?.InstallCACertificate() == true)
                {
                    MessageBox.Show(
                        "CA Certificate installed successfully!\n\n" +
                        "OISP Sensor can now intercept HTTPS traffic to AI APIs.",
                        "OISP Sensor",
                        MessageBoxButton.OK,
                        MessageBoxImage.Information);
                    UpdateTrayStatus();
                }
            }
            catch (Exception ex)
            {
                MessageBox.Show(
                    $"Failed to install CA certificate:\n{ex.Message}",
                    "OISP Sensor - Error",
                    MessageBoxButton.OK,
                    MessageBoxImage.Error);
            }
        }

        private void ShowCALocation_Click(object sender, RoutedEventArgs e)
        {
            var caPath = _certificateService?.GetCAPath();
            if (caPath != null && File.Exists(caPath))
            {
                // Open explorer and select the file
                System.Diagnostics.Process.Start("explorer.exe", $"/select,\"{caPath}\"");
            }
            else
            {
                var caDir = _certificateService?.GetCADirectory();
                if (caDir != null && Directory.Exists(caDir))
                {
                    System.Diagnostics.Process.Start("explorer.exe", caDir);
                }
                else
                {
                    MessageBox.Show(
                        "CA certificate not found. Start capture once to generate the CA.",
                        "OISP Sensor",
                        MessageBoxButton.OK,
                        MessageBoxImage.Information);
                }
            }
        }

        private void Settings_Click(object sender, RoutedEventArgs e)
        {
            if (_settingsWindow == null || !_settingsWindow.IsLoaded)
            {
                _settingsWindow = new SettingsWindow();
                _settingsWindow.SettingsChanged += OnSettingsChanged;
            }
            _settingsWindow.Show();
            _settingsWindow.Activate();
        }

        private void ViewLogs_Click(object sender, RoutedEventArgs e)
        {
            var settings = Settings.Load();
            var logDir = Path.GetDirectoryName(settings.OutputPath);
            if (logDir != null && Directory.Exists(logDir))
            {
                System.Diagnostics.Process.Start("explorer.exe", logDir);
            }
            else
            {
                MessageBox.Show(
                    $"Log directory not found:\n{logDir}",
                    "OISP Sensor",
                    MessageBoxButton.OK,
                    MessageBoxImage.Warning);
            }
        }

        private void Dashboard_Click(object sender, RoutedEventArgs e)
        {
            var settings = Settings.Load();
            var url = $"http://localhost:{settings.WebPort}";
            try
            {
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
                {
                    FileName = url,
                    UseShellExecute = true
                });
            }
            catch (Exception ex)
            {
                MessageBox.Show(
                    $"Failed to open dashboard:\n{ex.Message}\n\nMake sure the sensor is running with web server enabled.",
                    "OISP Sensor",
                    MessageBoxButton.OK,
                    MessageBoxImage.Warning);
            }
        }

        private void Exit_Click(object sender, RoutedEventArgs e)
        {
            Shutdown();
        }

        private void OnSettingsChanged(object? sender, EventArgs e)
        {
            // Settings were changed, may need to restart capture
            if (_isCapturing)
            {
                var result = MessageBox.Show(
                    "Settings have changed. Restart capture to apply changes?",
                    "OISP Sensor",
                    MessageBoxButton.YesNo,
                    MessageBoxImage.Question);

                if (result == MessageBoxResult.Yes)
                {
                    StopCapture();
                    StartCapture();
                }
            }
        }

        private void StartCapture()
        {
            if (_isCapturing) return;

            try
            {
                var settings = Settings.Load();

                // Start sensor process first
                _sensorService?.Start(settings);

                // Start redirector with elevation
                _redirectorService?.Start(settings);

                _isCapturing = true;
                UpdateTrayStatus();
                UpdateMenuState();
            }
            catch (Exception ex)
            {
                StopCapture();
                MessageBox.Show(
                    $"Failed to start capture:\n{ex.Message}",
                    "OISP Sensor - Error",
                    MessageBoxButton.OK,
                    MessageBoxImage.Error);
            }
        }

        private void StopCapture()
        {
            if (!_isCapturing) return;

            try
            {
                _redirectorService?.Stop();
                _sensorService?.Stop();
            }
            catch
            {
                // Ignore errors during shutdown
            }

            _isCapturing = false;
            UpdateTrayStatus();
            UpdateMenuState();
        }

        private void UpdateTrayStatus()
        {
            if (_trayIcon == null) return;

            var caInstalled = _certificateService?.IsCACertificateInstalled() ?? false;
            var status = _isCapturing ? "Capturing" : "Stopped";
            var caStatus = caInstalled ? "" : " (CA not installed)";

            _trayIcon.ToolTipText = $"OISP Sensor - {status}{caStatus}";

            // Update icon based on state
            var iconName = _isCapturing ? "oisp-icon-active.ico" : "oisp-icon.ico";
            try
            {
                _trayIcon.IconSource = new System.Windows.Media.Imaging.BitmapImage(
                    new Uri($"pack://application:,,,/Resources/{iconName}"));
            }
            catch
            {
                // Icon may not exist, ignore
            }
        }

        private void UpdateMenuState()
        {
            if (_trayIcon?.ContextMenu == null) return;

            foreach (var item in _trayIcon.ContextMenu.Items)
            {
                if (item is System.Windows.Controls.MenuItem menuItem)
                {
                    switch (menuItem.Name)
                    {
                        case "StartMenuItem":
                            menuItem.IsEnabled = !_isCapturing;
                            break;
                        case "StopMenuItem":
                            menuItem.IsEnabled = _isCapturing;
                            break;
                    }
                }
            }
        }
    }
}
