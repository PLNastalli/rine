import { fireEvent, render, screen } from '@testing-library/react';
import { expect, test } from 'vitest';
import { Diagnostics } from './Diagnostics';

const crash = {
  exe: '/apps/evil.exe',
  at: 1,
  report: {
    status: 'Crashed' as const,
    exit_code: null,
    stdout: '',
    stderr: 'guest log line (bloco RINE-CRASH capturado à parte)',
    crash: {
      version: '0.2.0-alpha.2',
      executable: 'evil.exe',
      phase: 'enter',
      reason: 'boom',
      location: 'x.rs:1',
      timestamp: 1700000000,
    },
    missing: [],
    duration_ms: 5,
    rine_version: '0.2.0-alpha.2',
  },
};

test('Diagnostics crash view shows structured fields, details on demand', async () => {
  render(<Diagnostics entries={[crash]} />);
  // Cabeçalho humano primeiro, nunca dump cru gigante.
  expect(await screen.findByText('Application crashed')).toBeInTheDocument();
  expect(screen.getByText('boom')).toBeInTheDocument();
  expect(screen.queryByText(/RINE-CRASH schema=1/)).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /Hide technical details/ })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: /Show technical details/ }));
  expect(await screen.findByRole('button', { name: /Hide technical details/ })).toBeInTheDocument();
});
