import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { EmptyState } from '@/components/EmptyState';
import { StatusDot } from '@/components/StatusDot';
import { Loading } from '@/components/Loading';
import { ErrorState } from '@/components/ErrorState';
import { CopyButton } from '@/components/CopyButton';
import { LocaleSwitcher } from '@/components/LocaleSwitcher';
import { ThemeToggle } from '@/components/ThemeToggle';
import { ApiError, NetworkError } from '@/api/errors';
import { useLocaleStore } from '@/stores/locale';
import { useThemeStore } from '@/stores/theme';
import { useToastsStore } from '@/stores/toasts';

const clipboard = vi.hoisted(() => ({ copy: vi.fn() }));

vi.mock('@/lib/clipboard', () => ({ copyToClipboard: clipboard.copy }));

beforeEach(() => {
  useToastsStore.getState().clear();
  useLocaleStore.setState({ locale: 'en' });
  useThemeStore.setState({ theme: 'system', effective: 'light' });
  clipboard.copy.mockReset();
  clipboard.copy.mockResolvedValue(true);
});

afterEach(() => {
  useToastsStore.getState().clear();
});

describe('EmptyState', () => {
  it('renders the default title and placeholder icon', () => {
    render(<EmptyState />);
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.getByText('Nothing here yet')).toBeInTheDocument();
  });

  it('renders a custom title, description, icon and action', () => {
    render(
      <EmptyState
        title="No repos"
        description="Create one to get started"
        icon={<span>ICON</span>}
        action={<button type="button">Create</button>}
      />,
    );
    expect(screen.getByText('No repos')).toBeInTheDocument();
    expect(screen.getByText('Create one to get started')).toBeInTheDocument();
    expect(screen.getByText('ICON')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create' })).toBeInTheDocument();
  });

  it('omits the description and action when not provided', () => {
    render(<EmptyState title="Bare" />);
    expect(screen.queryByText(/start/)).toBeNull();
  });
});

describe('StatusDot', () => {
  it.each([
    ['online', 'bg-emerald-500'],
    ['degraded', 'bg-amber-500'],
    ['offline', 'bg-rose-500'],
    ['unknown', 'bg-slate-400'],
  ] as const)('maps %s to its colour class', (state, className) => {
    const { container } = render(<StatusDot state={state} />);
    expect(container.querySelector(`.${className}`)).toBeInTheDocument();
  });

  it('renders the label only when provided', () => {
    const { rerender } = render(<StatusDot state="online" />);
    expect(screen.queryByText('Online')).toBeNull();
    rerender(<StatusDot state="online" label="Online" />);
    expect(screen.getByText('Online')).toBeInTheDocument();
  });
});

describe('Loading', () => {
  it('renders a polite status region with the default label', () => {
    render(<Loading />);
    const status = screen.getByRole('status');
    expect(status).toHaveAttribute('aria-live', 'polite');
    expect(screen.getByText('Loading…')).toBeInTheDocument();
  });

  it('renders a custom label and children', () => {
    render(
      <Loading label="Fetching repos">
        <button type="button">Cancel</button>
      </Loading>,
    );
    expect(screen.getByText('Fetching repos')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('switches to the fullscreen container class', () => {
    const { container } = render(<Loading fullscreen />);
    expect(container.firstChild).toHaveClass('fixed');
  });
});

describe('ErrorState', () => {
  it('explains a NetworkError with actionable text', () => {
    render(<ErrorState error={new NetworkError('refused', null)} />);
    expect(screen.getByText('Network error')).toBeInTheDocument();
    expect(screen.getByText(/gitgit-server is running/)).toBeInTheDocument();
  });

  it('shows an ApiError message and its code', () => {
    render(<ErrorState error={new ApiError(502, 'vault exploded', 'vault_error')} />);
    expect(screen.getByText('Request failed')).toBeInTheDocument();
    expect(screen.getByText('vault exploded')).toBeInTheDocument();
    expect(screen.getByText('code: vault_error')).toBeInTheDocument();
  });

  it('titles an unauthenticated ApiError differently', () => {
    render(<ErrorState error={new ApiError(401, 'token expired')} />);
    expect(screen.getByText('Authentication required')).toBeInTheDocument();
  });

  it('omits the code line when the ApiError carries no code', () => {
    render(<ErrorState error={new ApiError(400, 'bad input')} />);
    expect(screen.queryByText(/^code:/)).toBeNull();
  });

  it('surfaces a plain Error message', () => {
    render(<ErrorState error={new TypeError('kaboom')} />);
    expect(screen.getByText('Unexpected error')).toBeInTheDocument();
    expect(screen.getByText('kaboom')).toBeInTheDocument();
  });

  it('falls back to a generic body for a non-Error throwable', () => {
    render(<ErrorState error={'a string'} />);
    expect(screen.getByText('Unknown failure')).toBeInTheDocument();
  });

  it('lets the caller override the title and description', () => {
    render(
      <ErrorState
        error={new ApiError(500, 'ignored')}
        title="Custom title"
        description="Custom description"
      />,
    );
    expect(screen.getByText('Custom title')).toBeInTheDocument();
    expect(screen.getByText('Custom description')).toBeInTheDocument();
    expect(screen.queryByText('ignored')).toBeNull();
  });

  it('invokes onRetry when the retry button is pressed', async () => {
    const onRetry = vi.fn();
    render(<ErrorState error={new ApiError(500, 'x')} onRetry={onRetry} />);
    await userEvent.click(screen.getByRole('button', { name: 'Retry request' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('hides the retry button when no handler is supplied', () => {
    render(<ErrorState error={new ApiError(500, 'x')} />);
    expect(screen.queryByRole('button', { name: 'Retry request' })).toBeNull();
  });
});

describe('CopyButton', () => {
  it('copies the text and flips to the copied label', async () => {
    const user = userEvent.setup();
    render(<CopyButton text="secret-value" />);
    const button = screen.getByRole('button', { name: 'Copy' });
    await user.click(button);
    expect(clipboard.copy).toHaveBeenCalledWith('secret-value');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument());
  });

  it('cleans up its reset timer on unmount', async () => {
    const clearSpy = vi.spyOn(window, 'clearTimeout');
    const user = userEvent.setup();
    const { unmount } = render(<CopyButton text="x" />);
    await user.click(screen.getByRole('button', { name: 'Copy' }));
    unmount();
    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore();
  });

  it('raises an error toast when the copy is refused', async () => {
    clipboard.copy.mockResolvedValue(false);
    const user = userEvent.setup();
    render(<CopyButton text="x" />);
    await user.click(screen.getByRole('button', { name: 'Copy' }));
    const toasts = useToastsStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0]?.kind).toBe('error');
    expect(toasts[0]?.message).toMatch(/Copy failed/);
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument();
  });

  it('shows a success toast only when toastMessage is supplied', async () => {
    const user = userEvent.setup();
    const { rerender } = render(<CopyButton text="x" />);
    await user.click(screen.getByRole('button', { name: 'Copy' }));
    expect(useToastsStore.getState().toasts).toHaveLength(0);

    rerender(<CopyButton text="x" toastMessage="Key copied" />);
    await user.click(screen.getByRole('button', { name: 'Copied' }));
    expect(useToastsStore.getState().toasts[0]?.message).toBe('Key copied');
  });

  it('accepts custom labels and an extra class name', () => {
    render(<CopyButton text="x" label="Copy key" copiedLabel="Done" className="mt-2" />);
    const button = screen.getByRole('button', { name: 'Copy key' });
    expect(button).toHaveClass('mt-2');
  });
});

describe('LocaleSwitcher', () => {
  it('marks the active locale as pressed', () => {
    render(<LocaleSwitcher />);
    expect(screen.getByRole('button', { name: 'Language: en' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByRole('button', { name: 'Language: zh-CN' })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
  });

  it('switches the store locale on click', async () => {
    const user = userEvent.setup();
    render(<LocaleSwitcher />);
    await user.click(screen.getByRole('button', { name: 'Language: zh-CN' }));
    expect(useLocaleStore.getState().locale).toBe('zh-CN');
    expect(screen.getByRole('button', { name: 'Language: zh-CN' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
  });
});

describe('ThemeToggle', () => {
  it('exposes a radiogroup with the active theme checked', () => {
    render(<ThemeToggle />);
    expect(screen.getByRole('radiogroup', { name: 'Theme' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Theme: system' })).toHaveAttribute(
      'aria-checked',
      'true',
    );
    expect(screen.getByRole('radio', { name: 'Theme: dark' })).toHaveAttribute(
      'aria-checked',
      'false',
    );
  });

  it('persists the chosen theme through the store', async () => {
    const user = userEvent.setup();
    render(<ThemeToggle />);
    await user.click(screen.getByRole('radio', { name: 'Theme: dark' }));
    expect(useThemeStore.getState().theme).toBe('dark');
    expect(screen.getByRole('radio', { name: 'Theme: dark' })).toHaveAttribute(
      'aria-checked',
      'true',
    );
  });
});
