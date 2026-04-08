import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { lazy, Suspense } from 'react';
import { WalletProvider } from './providers/WalletProvider';
import Layout from './components/Layout';

// Lazy load route components for better initial bundle size
const Home = lazy(() => import('./pages/Home'));
const Signup = lazy(() => import('./pages/Signup'));
const Pricing = lazy(() => import('./pages/Pricing'));
const Dashboard = lazy(() => import('./pages/Dashboard'));
const Security = lazy(() => import('./pages/Security'));
const UseCases = lazy(() => import('./pages/UseCases'));
const Learn = lazy(() => import('./pages/Learn'));
const CourseDetail = lazy(() => import('./pages/CourseDetail'));

function App() {
  return (
    <Router>
      <WalletProvider>
        <Layout>
          <Suspense fallback={<div className="flex items-center justify-center min-h-screen"><div className="text-lg">Loading...</div></div>}>
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/signup" element={<Signup />} />
              <Route path="/pricing" element={<Pricing />} />
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/security" element={<Security />} />
              <Route path="/use-cases" element={<UseCases />} />
              <Route path="/learn" element={<Learn />} />
              <Route path="/learn/:slug" element={<CourseDetail />} />
            </Routes>
          </Suspense>
        </Layout>
      </WalletProvider>
    </Router>
  );
}

export default App;
