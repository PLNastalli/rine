import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, test, vi } from 'vitest';
import { Settings } from './Settings';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

const current = {
  schema: 1,
  appearance: 'dark',
  default_capsule_dir: '/c',
  confirm_destructive: true,
  log_level: 'info',
  retain_runs: 50,
  developer_mode: false,
};

beforeEach(() => {
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === 'get_settings') return Promise.resolve(current);
    if (cmd === 'save_settings') return Promise.resolve(undefined);
    return Promise.reject({ code: 'X', message: 'unexpected' });
  });
});

test('Settings save round-trips through the backend', async () => {
  render(<Settings apply={() => {}} />);
  expect(await screen.findByText('Appearance')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: /Save settings/ }));
  await screen.findByText('● Settings saved');
  const call = mockInvoke.mock.calls.find(([c]) => c === 'save_settings');
  expect(call).toBeDefined();
  expect((call![1] as { value: typeof current }).value).toEqual(current);
});
