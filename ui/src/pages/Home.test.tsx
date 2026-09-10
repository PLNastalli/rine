import { render, screen } from '@testing-library/react';
import { beforeEach, expect, test, vi } from 'vitest';
import { Home } from './Home';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === 'detect_runtime') return Promise.resolve({ version: '0.2.0-alpha.2', binary: '/r/rine', compatible: true });
    if (cmd === 'list_apps') return Promise.resolve([]);
    return Promise.reject({ code: 'X', message: 'unexpected' });
  });
});

test('Home renders runtime status + library empty state', async () => {
  render(<Home go={() => {}} />);
  expect(await screen.findByText(/Runtime Ready · 0\.2\.0-alpha\.2/)).toBeInTheDocument();
  expect(await screen.findByText('No Windows applications yet.')).toBeInTheDocument();
});
