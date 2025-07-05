#!/bin/bash

echo "🎪 Cyrus Protocol - Complete Demo"
echo "================================="
echo ""

echo "1️⃣ Deploying contracts..."
npm run deploy:all

echo ""
echo "2️⃣ Running end-to-end test..."
npm run test:e2e

echo ""
echo "3️⃣ Starting monitoring dashboard..."
echo "🔗 Dashboard will be available at http://localhost:3000"
npm run monitor:start &

echo ""
echo "🎉 Demo complete! Check the dashboard for live settlement tracking."
