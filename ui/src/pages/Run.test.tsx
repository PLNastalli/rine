import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, test, vi } from 'vitest';
import { Run } from './Run';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

beforeEach(() => {
  mockOpen.mockResolvedValue('/apps/suite.exe');
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === 'inspect_pe')
      return Promise.resolve({
        filename: 'suite.exe',
        size_bytes: 1234,
        machine: 0x8664,
        subsystem: 3,
        entry_point_rva: 0x1000,
        image_base: 0x140000000,
        imports: [{ dll: 'KERNEL32.dll', name: 'WriteFile', ordinal: null }],
        dll_dependencies: ['KERNEL32.dll'],
      });
    if (cmd === 'preflight')
      return Promise.resolve({ total_imports: 1, supported: 1, missing: [], verdict: 'ImportSurfaceSatisfied' });
    return Promise.reject({ code: 'X', message: 'unexpected' });
  });
});

test('Run selects executable, inspects and pre-flights (never runs alone)', async () => {
  render(<Run onDiagnosed={() => {}} />);
  fireEvent.click(screen.getByRole('button', { name: /Drop a Windows executable/i }));
  expect(await screen.findByText('/apps/suite.exe')).toBeInTheDocument();
  expect(await screen.findByText('Static import surface satisfied')).toBeInTheDocument();
  // Terminologia honesta: nunca "Compatible" por imports.
  expect(screen.queryByText('Compatible')).not.toBeInTheDocument();
  // Nada executou sozinho: run_exe jamais foi chamado.
  expect(mockInvoke.mock.calls.some(([c]) => c === 'run_exe')).toBe(false);
});
