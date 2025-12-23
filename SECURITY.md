# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

The OISP Sensor team takes security vulnerabilities seriously. We appreciate
your efforts to responsibly disclose your findings.

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to **security@oximy.com**.

Please include the following information in your report:

* Type of issue (e.g., buffer overflow, privilege escalation, information disclosure)
* Full paths of source file(s) related to the manifestation of the issue
* The location of the affected source code (tag/branch/commit or direct URL)
* Any special configuration required to reproduce the issue
* Step-by-step instructions to reproduce the issue
* Proof-of-concept or exploit code (if possible)
* Impact of the issue, including how an attacker might exploit it

You should receive a response within 48 hours. If for some reason you do not,
please follow up via email to ensure we received your original message.

## Preferred Languages

We prefer all communications to be in English.

## Disclosure Policy

When we receive a security bug report, we will:

1. Confirm the problem and determine affected versions
2. Audit code to find any similar problems
3. Prepare fixes for all supported versions
4. Release patches as soon as possible

## Security Considerations

OISP Sensor captures network traffic and system events, which may include
sensitive information. Users should:

* Run the sensor with minimal required privileges
* Use the redaction features to filter sensitive data
* Secure access to exported event logs
* Review captured data before sharing or storing