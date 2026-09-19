import { lazy, Suspense } from 'react';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { AppShell, ErrorBoundary, Loading, Toasts } from '@/components';

const Home = lazy(() => import('./routes/Home').then((m) => ({ default: m.Home })));
const RepoDetail = lazy(() => import('./routes/RepoDetail').then((m) => ({ default: m.RepoDetail })));
const Vault = lazy(() => import('./routes/Vault').then((m) => ({ default: m.Vault })));
const VaultKeyDetail = lazy(() =>
  import('./routes/VaultKeyDetail').then((m) => ({ default: m.VaultKeyDetail })),
);
const VaultDiff = lazy(() => import('./routes/VaultDiff').then((m) => ({ default: m.VaultDiff })));
const VaultRestore = lazy(() =>
  import('./routes/VaultRestore').then((m) => ({ default: m.VaultRestore })),
);
const Settings = lazy(() => import('./routes/Settings').then((m) => ({ default: m.Settings })));
const NotFound = lazy(() => import('./routes/NotFound').then((m) => ({ default: m.NotFound })));

const LAZY_FALLBACK = (
  <div className="py-12">
    <Loading label="Loading…" />
  </div>
);

function Topbar({ title }: { title: string }) {
  return <span className="truncate text-sm font-medium">{title}</span>;
}

/**
 * Application root. Hosts the router, app shell, and top-level
 * error boundary. Route components are code-split for fast first
 * paint; each route also wraps its content in its own ErrorBoundary
 * for granular failure isolation.
 */
export function App() {
  return (
    <BrowserRouter>
      <ErrorBoundary scope="App">
        <AppShell
          topbar={
            <Routes>
              <Route path="/" element={<Topbar title="Repositories" />} />
              <Route path="/repos/:name" element={<Topbar title="Repository" />} />
              <Route path="/vault" element={<Topbar title="Vault" />} />
              <Route path="/vault/:key" element={<Topbar title="Vault key" />} />
              <Route path="/vault/:key/diff" element={<Topbar title="Diff" />} />
              <Route path="/vault/:key/restore" element={<Topbar title="Restore" />} />
              <Route path="/settings" element={<Topbar title="Settings" />} />
            </Routes>
          }
        >
          <Suspense fallback={LAZY_FALLBACK}>
            <Routes>
              <Route
                path="/"
                element={
                  <ErrorBoundary scope="route:home">
                    <Home />
                  </ErrorBoundary>
                }
              />
              <Route
                path="/repos/:name"
                element={
                  <ErrorBoundary scope="route:repo-detail">
                    <RepoDetail />
                  </ErrorBoundary>
                }
              />
              <Route
                path="/vault"
                element={
                  <ErrorBoundary scope="route:vault">
                    <Vault />
                  </ErrorBoundary>
                }
              />
              <Route
                path="/vault/:key/diff"
                element={
                  <ErrorBoundary scope="route:vault-diff">
                    <VaultDiff />
                  </ErrorBoundary>
                }
              />
              <Route
                path="/vault/:key/restore"
                element={
                  <ErrorBoundary scope="route:vault-restore">
                    <VaultRestore />
                  </ErrorBoundary>
                }
              />
              <Route
                path="/vault/:key"
                element={
                  <ErrorBoundary scope="route:vault-key">
                    <VaultKeyDetail />
                  </ErrorBoundary>
                }
              />
              <Route
                path="/settings"
                element={
                  <ErrorBoundary scope="route:settings">
                    <Settings />
                  </ErrorBoundary>
                }
              />
              <Route path="/404" element={<NotFound />} />
              <Route path="*" element={<Navigate to="/404" replace />} />
            </Routes>
          </Suspense>
        </AppShell>
        <Toasts />
      </ErrorBoundary>
    </BrowserRouter>
  );
}