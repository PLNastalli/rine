import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, test, vi } from 'vitest';
import { Capsules } from './Capsules';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === 'list_capsules')
      return Promise.resolve([
        { path: '/c/game/capsule.toml', name: 'game', windows_version: [10, 0], drives: ['C'], valid: true, error: null },
      ]);
    if (cmd === 'create_capsule') return Promise.resolve('/c/game2/capsule.toml');
    return Promise.reject({ code: 'X', message: 'unexpected' });
  });
});

test('Capsule creation sends only schema-v0.2 keys', async () => {
  render(<Capsules />);
  fireEvent.click(screen.getByRole('button', { name: /Create Capsule/ }));
  const dir = screen.getByPlaceholderText('~/Rine/capsules');
  fireEvent.change(dir, { target: { value: '/c' } });
  fireEvent.change(screen.getByPlaceholderText('game'), { target: { value: 'game2' } });
  fireEvent.change(screen.getByPlaceholderText('~/Rine/capsules/game/drive_c'), {
    target: { value: '/c/game2/drive_c' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^Create$/ }));
  await screen.findByText('game');
  const call = mockInvoke.mock.calls.find(([c]) => c === 'create_capsule');
  expect(call).toBeDefined();
  const payload = (call![1] as { payload: Record<string, unknown> }).payload;
  // Nenhuma chave inventada: só o que o runtime lê.
  expect(Object.keys(payload).sort()).toEqual(['current_dir', 'drives', 'name', 'parent_dir']);
  expect(payload).toEqual({
    name: 'game2',
    parent_dir: '/c',
    drives: [['C', '/c/game2/drive_c']],
    current_dir: null,
  });
});
