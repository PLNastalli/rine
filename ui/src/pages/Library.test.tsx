import { render, screen } from '@testing-library/react';
import { beforeEach, expect, test, vi } from 'vitest';
import { Library } from './Library';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === 'list_apps') return Promise.resolve([]);
    return Promise.reject({ code: 'X', message: 'unexpected' });
  });
});

test('Library empty state explains the next step', async () => {
  render(
    <Library
      go={() => {}}
      onDiagnosed={() => {}}
    />,
  );
  expect(await screen.findByText('No Windows applications yet.')).toBeInTheDocument();
  expect(await screen.findByText('Add an executable to get started.')).toBeInTheDocument();
});
