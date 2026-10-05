import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { Sidebar } from '@/components/Sidebar';
import { AppShell } from '@/components/AppShell';
import { Toasts } from '@/components/Toast';
import { ErrorBoundary, useToast } from '@/components/ErrorBoundary';
import { useToastsStore } from '@/stores/toasts';
import { useLocaleStore } from '@/stores/locale';
import { useThemeStore } from '@/stores/theme';

function renderInRouter(ui: React.ReactElement, path = '/') {
  return render(<MemoryRouter initialEntries={[path]}>{ui}</MemoryRouter>);
}

beforeEach(() => {
  useToastsStore.getState().clear();
  useLocaleStore.setState({ locale: 'en' });
  useThemeStore.setState({ theme: 'system', effective: 'light' });
});

afterEach(() => {
  // `[FACT]` `<Toasts>` is still mounted here — `cleanup()` is registered
  // in `vitest.setup.ts`, which loads first, and vitest's default
  // `sequence.hooks: 'stack'` runs this later-registered hook first. A
  // bare `clear()` therefore re-renders a live component outside React's
  // act environment.
  act(() => useToastsStore.getState().clear());
});

describe('Sidebar', () => {
  it('renders all three nav sections', () => {
    renderInRouter(<Sidebar open={false} onClose={() => {}} />);
    const nav = screen.getByRole('navigation', { name: 'Sections' });
    expect(nav).toBeInTheDocument();
    expect(screen.getAllByRole('link')).toHaveLength(3);
  });

  it('marks the repos link as the current page at the root', () => {
    renderInRouter(<Sidebar open={false} onClose={() => {}} />, '/');
    const links = screen.getAllByRole('link');
    expect(links[0]).toHaveAttribute('aria-current', 'page');
  });

  it('marks the vault link as current inside the vault section', () => {
    renderInRouter(<Sidebar open={false} onClose={() => {}} />, '/vault/keys/openai');
    const links = screen.getAllByRole('link');
    expect(links[0]).not.toHaveAttribute('aria-current', 'page');
    expect(links[1]).toHaveAttribute('aria-current', 'page');
  });

  it('marks the settings link as current inside the settings section', () => {
    renderInRouter(<Sidebar open={false} onClose={() => {}} />, '/settings');
    const links = screen.getAllByRole('link');
    expect(links[2]).toHaveAttribute('aria-current', 'page');
  });

  it('invokes onClose when a nav link is clicked', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    renderInRouter(<Sidebar open onClose={onClose} />);
    await user.click(screen.getAllByRole('link')[0]!);
    expect(onClose).toHaveBeenCalled();
  });

  it('slides the panel off-canvas when closed', () => {
    const { rerender } = renderInRouter(<Sidebar open={false} onClose={() => {}} />);
    expect(screen.getByRole('complementary')).toHaveClass('-translate-x-full');
    rerender(
      <MemoryRouter initialEntries={['/']}>
        <Sidebar open onClose={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByRole('complementary')).toHaveClass('translate-x-0');
  });

  it('renders the locale and theme controls in the footer', () => {
    renderInRouter(<Sidebar open={false} onClose={() => {}} />);
    expect(screen.getByRole('group', { name: 'Language' })).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: 'Theme' })).toBeInTheDocument();
  });
});

describe('AppShell', () => {
  it('renders children inside the main region', () => {
    renderInRouter(
      <AppShell>
        <p>page body</p>
      </AppShell>,
    );
    expect(screen.getByText('page body')).toBeInTheDocument();
  });

  it('renders optional topbar content', () => {
    renderInRouter(<AppShell topbar={<span>breadcrumb / repos</span>}>{null}</AppShell>);
    expect(screen.getByText('breadcrumb / repos')).toBeInTheDocument();
  });

  it('starts with the drawer closed', () => {
    renderInRouter(<AppShell>{null}</AppShell>);
    expect(screen.getByRole('button', { name: 'Open menu' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
  });

  it('toggles the drawer open from the topbar button', async () => {
    const user = userEvent.setup();
    renderInRouter(<AppShell>{null}</AppShell>);
    await user.click(screen.getByRole('button', { name: 'Open menu' }));
    expect(screen.getByRole('button', { name: 'Close menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
  });

  it('closes the drawer on Escape', async () => {
    const user = userEvent.setup();
    renderInRouter(<AppShell>{null}</AppShell>);
    await user.click(screen.getByRole('button', { name: 'Open menu' }));
    expect(screen.getByRole('button', { name: 'Close menu' })).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.getByRole('button', { name: 'Open menu' })).toBeInTheDocument();
  });
});

describe('Toasts', () => {
  it('renders an empty live region when there are no toasts', () => {
    const { container } = render(<Toasts />);
    expect(container.querySelector('[aria-live="polite"]')).toBeEmptyDOMElement();
  });

  it('renders each active toast with its message', () => {
    useToastsStore.getState().push('info', 'first');
    useToastsStore.getState().push('success', 'second');
    render(<Toasts />);
    expect(screen.getByText('first')).toBeInTheDocument();
    expect(screen.getByText('second')).toBeInTheDocument();
  });

  it('exposes errors as alerts and everything else as status', () => {
    useToastsStore.getState().push('error', 'boom');
    useToastsStore.getState().push('warning', 'careful');
    render(<Toasts />);
    expect(screen.getByRole('alert')).toHaveTextContent('boom');
    expect(screen.getByRole('status')).toHaveTextContent('careful');
  });

  it('dismisses a toast when its close button is pressed', async () => {
    const user = userEvent.setup();
    useToastsStore.getState().push('info', 'dismiss me');
    render(<Toasts />);
    await user.click(screen.getByRole('button', { name: 'Dismiss notification' }));
    expect(useToastsStore.getState().toasts).toHaveLength(0);
  });
});

/** A component that always throws, to drive the boundary. */
function Boom({ error }: { error: unknown }): React.ReactElement {
  throw error;
}

describe('ErrorBoundary', () => {
  let consoleError: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // React logs the caught error; keep the test output readable.
    consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleError.mockRestore();
  });

  it('renders children when nothing throws', () => {
    render(
      <ErrorBoundary>
        <p>all good</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText('all good')).toBeInTheDocument();
  });

  it('renders a fallback with the error message', () => {
    render(
      <ErrorBoundary>
        <Boom error={new Error('render exploded')} />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('render exploded')).toBeInTheDocument();
  });

  it('uses the scope as the fallback heading', () => {
    render(
      <ErrorBoundary scope="Vault page">
        <Boom error={new Error('x')} />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('heading', { name: 'Vault page' })).toBeInTheDocument();
  });

  it('falls back to a default heading without a scope', () => {
    render(
      <ErrorBoundary>
        <Boom error={new Error('x')} />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('heading', { name: 'Render error' })).toBeInTheDocument();
  });

  it('stringifies a non-Error throwable', () => {
    render(
      <ErrorBoundary>
        <Boom error={'plain string failure'} />
      </ErrorBoundary>,
    );
    expect(screen.getByText('plain string failure')).toBeInTheDocument();
  });

  it('logs the failure with its scope', () => {
    render(
      <ErrorBoundary scope="Repos page">
        <Boom error={new Error('logged')} />
      </ErrorBoundary>,
    );
    // React itself logs the uncaught error first, so look for the
    // boundary's own line rather than assuming it is call #0.
    const boundaryCall = consoleError.mock.calls.find(
      (call) => typeof call[0] === 'string' && call[0] === '[ErrorBoundary]',
    );
    expect(boundaryCall).toBeDefined();
    expect(boundaryCall?.[1]).toBe('Repos page');
    expect(boundaryCall?.[2]).toBeInstanceOf(Error);
  });

  it('offers a reload button that calls window.location.reload', () => {
    const reload = vi.fn();
    const original = window.location;
    Object.defineProperty(window, 'location', {
      writable: true,
      configurable: true,
      value: { ...original, reload },
    });

    render(
      <ErrorBoundary>
        <Boom error={new Error('x')} />
      </ErrorBoundary>,
    );
    const button = screen.getByRole('button', { name: 'Reload page' });
    expect(button).toBeInTheDocument();
    button.click();
    expect(reload).toHaveBeenCalled();

    Object.defineProperty(window, 'location', {
      writable: true,
      configurable: true,
      value: original,
    });
  });
});

describe('useToast', () => {
  function Trigger() {
    const { push } = useToast();
    return (
      <button type="button" onClick={() => push('info', 'from hook')}>
        push
      </button>
    );
  }

  it('pushes a toast through the store', async () => {
    const user = userEvent.setup();
    render(<Trigger />);
    await user.click(screen.getByRole('button', { name: 'push' }));
    expect(useToastsStore.getState().toasts[0]?.message).toBe('from hook');
  });
});
