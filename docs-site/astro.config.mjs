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
      ],
      components: {
        Head: './src/components/Head.astro',
      },
    }),
  ],
});

