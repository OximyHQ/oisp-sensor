import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'OISP Sensor',
  description: 'AI Agent Observability - See what your AI agents are doing',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-bg-primary text-text-primary min-h-screen">
        {children}
      </body>
    </html>
  );
}

