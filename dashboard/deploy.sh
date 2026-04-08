#!/bin/bash
set -e

echo "🚀 Building Parapet Dashboard for production..."

# Build production bundle
npm run build

echo "✅ Production build complete!"
echo ""
echo "📦 Deployment options:"
echo ""
echo "1. Static files (dist/):"
echo "   - Copy 'dist/' folder to your web server"
echo "   - Serve with nginx, Apache, or any static file server"
echo ""
echo "2. Docker:"
echo "   docker build -t parapet-dashboard ."
echo "   docker run -p 8080:80 parapet-dashboard"
echo ""
echo "3. Docker Compose:"
echo "   cd deployments/dashboard"
echo "   docker-compose up -d"
echo ""
echo "4. Quick test (Python):"
echo "   cd dist && python3 -m http.server 8080"
echo ""
echo "📁 Production files: dist/"
echo "📊 Bundle size: $(du -sh dist/ | cut -f1)"
