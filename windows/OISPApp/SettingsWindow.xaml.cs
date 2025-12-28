using Microsoft.Win32;
using System;
using System.Windows;

namespace OISPApp
{
    /// <summary>
    /// Settings window for OISP Sensor configuration
    /// </summary>
    public partial class SettingsWindow : Window
    {
        private Settings _settings;

        /// <summary>
        /// Event raised when settings are saved
        /// </summary>
        public event EventHandler? SettingsChanged;

        public SettingsWindow()
        {
            InitializeComponent();
            _settings = Settings.Load();
            LoadSettingsToUI();
        }

        private void LoadSettingsToUI()
        {
            OutputPathTextBox.Text = _settings.OutputPath;
            TlsMitmCheckBox.IsChecked = _settings.EnableTlsMitm;
            AiFilterCheckBox.IsChecked = _settings.AiEndpointFilterEnabled;
            ProcessFilterTextBox.Text = _settings.ProcessFilter;
            ProxyPortTextBox.Text = _settings.ProxyPort.ToString();
            AutoStartCheckBox.IsChecked = _settings.AutoStartCapture;
            VerboseCheckBox.IsChecked = _settings.VerboseLogging;
            SensorPathTextBox.Text = _settings.SensorPath;
            RedirectorPathTextBox.Text = _settings.RedirectorPath;
        }

        private bool SaveSettingsFromUI()
        {
            // Validate port
            if (!int.TryParse(ProxyPortTextBox.Text, out int port) || port < 1 || port > 65535)
            {
                MessageBox.Show(
                    "Please enter a valid port number (1-65535).",
                    "Validation Error",
                    MessageBoxButton.OK,
                    MessageBoxImage.Warning);
                ProxyPortTextBox.Focus();
                return false;
            }

            // Validate output path
            if (string.IsNullOrWhiteSpace(OutputPathTextBox.Text))
            {
                MessageBox.Show(
                    "Please specify an output path.",
                    "Validation Error",
                    MessageBoxButton.OK,
                    MessageBoxImage.Warning);
                OutputPathTextBox.Focus();
                return false;
            }

            _settings.OutputPath = OutputPathTextBox.Text.Trim();
            _settings.EnableTlsMitm = TlsMitmCheckBox.IsChecked ?? true;
            _settings.AiEndpointFilterEnabled = AiFilterCheckBox.IsChecked ?? true;
            _settings.ProcessFilter = ProcessFilterTextBox.Text.Trim();
            _settings.ProxyPort = port;
            _settings.AutoStartCapture = AutoStartCheckBox.IsChecked ?? false;
            _settings.VerboseLogging = VerboseCheckBox.IsChecked ?? false;
            _settings.SensorPath = SensorPathTextBox.Text.Trim();
            _settings.RedirectorPath = RedirectorPathTextBox.Text.Trim();

            _settings.Save();
            return true;
        }

        private void BrowseOutput_Click(object sender, RoutedEventArgs e)
        {
            var dialog = new SaveFileDialog
            {
                Title = "Select Output File",
                Filter = "JSON Lines (*.jsonl)|*.jsonl|All Files (*.*)|*.*",
                DefaultExt = ".jsonl",
                FileName = System.IO.Path.GetFileName(OutputPathTextBox.Text)
            };

            var initialDir = System.IO.Path.GetDirectoryName(OutputPathTextBox.Text);
            if (!string.IsNullOrEmpty(initialDir) && System.IO.Directory.Exists(initialDir))
            {
                dialog.InitialDirectory = initialDir;
            }

            if (dialog.ShowDialog() == true)
            {
                OutputPathTextBox.Text = dialog.FileName;
            }
        }

        private void ResetDefaults_Click(object sender, RoutedEventArgs e)
        {
            var result = MessageBox.Show(
                "Reset all settings to defaults?",
                "Confirm Reset",
                MessageBoxButton.YesNo,
                MessageBoxImage.Question);

            if (result == MessageBoxResult.Yes)
            {
                _settings = new Settings();
                LoadSettingsToUI();
            }
        }

        private void Cancel_Click(object sender, RoutedEventArgs e)
        {
            Close();
        }

        private void Save_Click(object sender, RoutedEventArgs e)
        {
            if (SaveSettingsFromUI())
            {
                SettingsChanged?.Invoke(this, EventArgs.Empty);
                Close();
            }
        }
    }
}
