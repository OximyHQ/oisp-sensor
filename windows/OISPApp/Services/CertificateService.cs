using System;
using System.Diagnostics;
using System.IO;
using System.Security.Cryptography.X509Certificates;

namespace OISPApp.Services
{
    /// <summary>
    /// Manages OISP CA certificate installation
    /// </summary>
    public class CertificateService
    {
        private const string CA_FILENAME = "oisp-ca.crt";
        private const string CA_SUBJECT = "CN=OISP Sensor CA";

        /// <summary>
        /// Get the OISP data directory
        /// </summary>
        public string GetCADirectory()
        {
            var localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            return Path.Combine(localAppData, "OISP");
        }

        /// <summary>
        /// Get the CA certificate path
        /// </summary>
        public string GetCAPath()
        {
            return Path.Combine(GetCADirectory(), CA_FILENAME);
        }

        /// <summary>
        /// Check if CA certificate exists on disk
        /// </summary>
        public bool CACertificateExists()
        {
            return File.Exists(GetCAPath());
        }

        /// <summary>
        /// Check if CA certificate is installed in the Trusted Root store
        /// </summary>
        public bool IsCACertificateInstalled()
        {
            try
            {
                using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
                store.Open(OpenFlags.ReadOnly);

                foreach (var cert in store.Certificates)
                {
                    if (cert.Subject.Contains("OISP Sensor CA"))
                    {
                        return true;
                    }
                }

                return false;
            }
            catch
            {
                return false;
            }
        }

        /// <summary>
        /// Install the CA certificate to the Trusted Root store
        /// </summary>
        public bool InstallCACertificate()
        {
            var caPath = GetCAPath();

            if (!File.Exists(caPath))
            {
                throw new FileNotFoundException(
                    "CA certificate not found. Start capture once to generate it.",
                    caPath);
            }

            try
            {
                // Load the certificate
                var cert = new X509Certificate2(caPath);

                // Open the Trusted Root store for current user
                using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
                store.Open(OpenFlags.ReadWrite);

                // Check if already installed
                if (IsCertificateInStore(store, cert))
                {
                    return true; // Already installed
                }

                // Add the certificate
                store.Add(cert);

                return true;
            }
            catch (Exception ex)
            {
                // If programmatic installation fails, try using certutil
                return InstallCACertificateWithCertutil(caPath);
            }
        }

        /// <summary>
        /// Remove the CA certificate from the Trusted Root store
        /// </summary>
        public bool RemoveCACertificate()
        {
            try
            {
                using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
                store.Open(OpenFlags.ReadWrite);

                var toRemove = new X509Certificate2Collection();
                foreach (var cert in store.Certificates)
                {
                    if (cert.Subject.Contains("OISP Sensor CA"))
                    {
                        toRemove.Add(cert);
                    }
                }

                foreach (var cert in toRemove)
                {
                    store.Remove(cert);
                }

                return true;
            }
            catch
            {
                return false;
            }
        }

        /// <summary>
        /// Open the certificate manager for manual installation
        /// </summary>
        public void OpenCertificateManager()
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = "certmgr.msc",
                UseShellExecute = true
            });
        }

        /// <summary>
        /// Open the CA certificate with the default handler (cert install wizard)
        /// </summary>
        public void OpenCACertificateWizard()
        {
            var caPath = GetCAPath();
            if (File.Exists(caPath))
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = caPath,
                    UseShellExecute = true
                });
            }
        }

        /// <summary>
        /// Install certificate using certutil.exe
        /// </summary>
        private bool InstallCACertificateWithCertutil(string certPath)
        {
            try
            {
                var startInfo = new ProcessStartInfo
                {
                    FileName = "certutil.exe",
                    Arguments = $"-user -addstore Root \"{certPath}\"",
                    UseShellExecute = true,
                    Verb = "runas", // May need elevation
                    CreateNoWindow = true
                };

                var process = Process.Start(startInfo);
                process?.WaitForExit(30000);

                return process?.ExitCode == 0;
            }
            catch
            {
                return false;
            }
        }

        /// <summary>
        /// Check if a certificate is already in a store
        /// </summary>
        private bool IsCertificateInStore(X509Store store, X509Certificate2 cert)
        {
            foreach (var existing in store.Certificates)
            {
                if (existing.Thumbprint == cert.Thumbprint)
                {
                    return true;
                }
            }
            return false;
        }
    }
}
