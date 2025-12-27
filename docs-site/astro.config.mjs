import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://sensor.oisp.dev',
  integrations: [
    starlight({
      title: 'OISP Sensor',
      description: 'Universal AI Observability - zero-instrumentation monitoring for AI systems',
      logo: {
        light: './src/assets/logo-light.svg',
        dark: './src/assets/logo-dark.svg',
        replacesTitle: false,
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/oximyHQ/oisp-sensor' },
      ],
      editLink: {
        baseUrl: 'https://github.com/oximyHQ/oisp-sensor/edit/main/docs-site/',
      },
      customCss: [
        './src/styles/custom.css',
      ],
      head: [
        {
          tag: 'meta',
          attrs: {
            property: 'og:image',
            content: 'https://sensor.oisp.dev/og-image.png',
          },
        },
        {
          tag: 'script',
          attrs: {
            defer: true,
            'data-domain': 'sensor.oisp.dev',
            src: 'https://plausible.io/js/script.js',
          },
        },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', slug: 'getting-started/introduction' },
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'Quick Start', slug: 'getting-started/quick-start' },
            { label: 'What Works Today', slug: 'getting-started/what-works-today' },
          ],
        },
        {
          label: 'Platforms',
          items: [
            {
              label: 'Linux',
              collapsed: false,
              items: [
                { label: 'Overview', slug: 'platforms/linux/overview' },
                { label: 'Installation', slug: 'platforms/linux/installation' },
                { label: 'Quick Start', slug: 'platforms/linux/quick-start' },
                { label: 'Running as a Service', slug: 'platforms/linux/service' },
                { label: 'Production Deployment', slug: 'platforms/linux/production' },
                { label: 'Distribution Support', slug: 'platforms/linux/distributions' },
                { label: 'Troubleshooting', slug: 'platforms/linux/troubleshooting' },
              ],
            },
            {
              label: 'macOS',
              collapsed: true,
              items: [
                { label: 'Overview', slug: 'platforms/macos/overview' },
                { label: 'Installation', slug: 'platforms/macos/installation' },
                { label: 'Quick Start', slug: 'platforms/macos/quick-start' },
                { label: 'Limitations', slug: 'platforms/macos/limitations' },
              ],
            },
            {
              label: 'Windows',
              collapsed: true,
              items: [
                { label: 'Overview', slug: 'platforms/windows/overview' },
                { label: 'Installation', slug: 'platforms/windows/installation' },
                { label: 'Quick Start', slug: 'platforms/windows/quick-start' },
              ],
            },
            {
              label: 'Docker',
              collapsed: true,
              items: [
                { label: 'Overview', slug: 'platforms/docker/overview' },
                { label: 'Running with Docker', slug: 'platforms/docker/running' },
                { label: 'Docker Compose', slug: 'platforms/docker/compose' },
              ],
            },
            {
              label: 'Kubernetes',
              collapsed: true,
              items: [
                { label: 'Overview', slug: 'platforms/kubernetes/overview' },
                { label: 'DaemonSet Deployment', slug: 'platforms/kubernetes/daemonset' },
                { label: 'Centralized Logging', slug: 'platforms/kubernetes/logging' },
              ],
            },
          ],
        },
        {
          label: 'Cookbooks',
          items: [
            { label: 'Overview', slug: 'cookbooks/overview' },
            {
              label: 'Python',
              collapsed: true,
              items: [
                { label: 'OpenAI Simple', slug: 'cookbooks/python/openai-simple' },
                { label: 'LiteLLM', slug: 'cookbooks/python/litellm' },
                { label: 'LangChain Agent', slug: 'cookbooks/python/langchain-agent' },
                { label: 'FastAPI Service', slug: 'cookbooks/python/fastapi-service' },
              ],
            },
            {
              label: 'Node.js',
              collapsed: true,
              items: [
                { label: 'OpenAI Simple', slug: 'cookbooks/node/openai-simple' },
              ],
            },
            {
              label: 'Self-Hosted',
              collapsed: true,
              items: [
                { label: 'n8n', slug: 'cookbooks/self-hosted/n8n' },
              ],
            },
            {
              label: 'Multi-Process',
              collapsed: true,
              items: [
                { label: 'Python Celery', slug: 'cookbooks/multi-process/celery' },
              ],
            },
            {
              label: 'Kubernetes',
              collapsed: true,
              items: [
                { label: 'DaemonSet', slug: 'cookbooks/kubernetes/daemonset' },
              ],
            },
            {
              label: 'Edge Cases',
              collapsed: true,
              items: [
                { label: 'NVM Node.js', slug: 'cookbooks/edge-cases/nvm-node' },
                { label: 'pyenv Python', slug: 'cookbooks/edge-cases/pyenv-python' },
              ],
            },
          ],
        },
        {
          label: 'Architecture',
          items: [
            { label: 'Overview', slug: 'architecture/overview' },
            { label: 'eBPF Capture', slug: 'architecture/ebpf' },
            { label: 'Pipeline', slug: 'architecture/pipeline' },
            { label: 'Event Schema', slug: 'architecture/events' },
          ],
        },
        {
          label: 'Configuration',
          items: [
            { label: 'Config File', slug: 'configuration/config-file' },
            { label: 'Exports', slug: 'configuration/exports' },
            { label: 'Filters', slug: 'configuration/filters' },
            { label: 'Redaction', slug: 'configuration/redaction' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI Commands', slug: 'reference/cli' },
            { label: 'API Reference', slug: 'reference/api' },
            { label: 'OISP Spec', slug: 'reference/oisp-spec' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Troubleshooting', slug: 'guides/troubleshooting' },
            { label: 'Multi-Node Deployment', slug: 'guides/multi-node' },
            { label: 'CI/CD Integration', slug: 'guides/ci-cd' },
            { label: 'Contributing', slug: 'guides/contributing' },
          ],
        },
      ],
      components: {
        Head: './src/components/Head.astro',
      },
    }),
  ],
});

