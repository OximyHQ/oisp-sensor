/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
  // For embedding in Rust binary
  distDir: 'out',
};

module.exports = nextConfig;

