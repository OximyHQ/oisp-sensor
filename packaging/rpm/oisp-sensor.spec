Name:           oisp-sensor
Version:        0.2.0
Release:        1%{?dist}
Summary:        Universal AI Observability Sensor
License:        Apache-2.0
URL:            https://sensor.oisp.dev
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  systemd-rpm-macros
Requires:       glibc >= 2.31
Requires:       libcap
Requires(post): systemd
Requires(preun): systemd
Requires(postun): systemd

%description
OISP Sensor captures AI system activity including:
 - LLM API calls (OpenAI, Anthropic, Gemini, etc.)
 - Process execution and hierarchy
 - File operations
 - Network connections

Features:
 - Zero-instrumentation using eBPF on Linux
 - Real-time web UI dashboard
 - JSONL, WebSocket, OTLP, Kafka export
 - Privacy-first with configurable redaction
 - OISP v0.1 spec compliant events

%prep
%setup -q

%build
# Binary is pre-built in the tarball
# If building from source, use: cargo build --release

%install
rm -rf %{buildroot}

# Install binary
install -D -m 0755 oisp-sensor %{buildroot}%{_bindir}/oisp-sensor

# Install systemd service
install -D -m 0644 packaging/systemd/oisp-sensor.service %{buildroot}%{_unitdir}/oisp-sensor.service

# Create directories
install -d -m 0755 %{buildroot}%{_sysconfdir}/oisp
install -d -m 0755 %{buildroot}%{_localstatedir}/log/oisp
install -d -m 0755 %{buildroot}%{_localstatedir}/lib/oisp

%pre
# Create oisp user and group
getent group oisp >/dev/null || groupadd -r oisp
getent passwd oisp >/dev/null || \
    useradd -r -g oisp -d /var/lib/oisp -s /sbin/nologin \
    -c "OISP Sensor Service" oisp
exit 0

%post
# Set capabilities for eBPF
if [ -x /usr/sbin/setcap ]; then
    /usr/sbin/setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep %{_bindir}/oisp-sensor || :
fi

# Create default config if it doesn't exist
if [ ! -f %{_sysconfdir}/oisp/config.toml ]; then
    cat > %{_sysconfdir}/oisp/config.toml << 'EOF'
# OISP Sensor Configuration
# See https://sensor.oisp.dev/configuration for documentation

[sensor]
name = "oisp-sensor"

[capture]
ssl = true
process = true
file = true
network = true

[redaction]
mode = "safe"  # safe, full, minimal

[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
append = true

[web]
enabled = true
host = "127.0.0.1"
port = 7777
EOF
    chown root:oisp %{_sysconfdir}/oisp/config.toml
    chmod 640 %{_sysconfdir}/oisp/config.toml
fi

# Set directory ownership
chown oisp:oisp %{_localstatedir}/log/oisp
chown oisp:oisp %{_localstatedir}/lib/oisp

# Systemd integration
%systemd_post oisp-sensor.service

# Print installation message
cat << 'EOFMSG'

OISP Sensor installed successfully!

Get started:
  sudo systemctl enable oisp-sensor   # Enable on boot
  sudo systemctl start oisp-sensor    # Start now
  oisp-sensor status                  # Check capabilities

Web UI: http://localhost:7777
Config: /etc/oisp/config.toml

EOFMSG

%preun
%systemd_preun oisp-sensor.service

%postun
%systemd_postun_with_restart oisp-sensor.service

# Clean up user/group on package removal (not upgrade)
if [ $1 -eq 0 ]; then
    userdel oisp >/dev/null 2>&1 || :
    groupdel oisp >/dev/null 2>&1 || :
fi

%files
%{_bindir}/oisp-sensor
%{_unitdir}/oisp-sensor.service
%dir %attr(0755,root,root) %{_sysconfdir}/oisp
%dir %attr(0755,oisp,oisp) %{_localstatedir}/log/oisp
%dir %attr(0755,oisp,oisp) %{_localstatedir}/lib/oisp
%config(noreplace) %{_sysconfdir}/oisp/config.toml

%changelog
* Thu Dec 26 2024 Oximy Team <team@oximy.com> - 0.2.0-1
- Initial RPM release
- eBPF-based SSL/TLS capture
- Multi-export support (JSONL, WebSocket, OTLP, Kafka, Webhook)
- Real-time web UI dashboard
- Systemd service integration
- RHEL/CentOS/Fedora support
