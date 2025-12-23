import './globals.css';
import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'OISP Sensor - AI Agent Observability',
  description: 'Real-time monitoring and observability for AI agents and LLM applications',
  icons: {
    icon: '/favicon.ico',
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="bg-bg-primary text-text-primary antialiased">
        {children}
      </body>
    </html>
  );
}
