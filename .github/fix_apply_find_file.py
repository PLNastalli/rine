from pathlib import Path

p = Path('.github/apply_find_file.py')
text = p.read_text()
start = text.index('# CloseHandle must not consume')
end = text.index('# ntdll typed bridge.')
text = text[:start] + '# CloseHandle semantics are hardened in the follow-up patch.\n\n' + text[end:]
p.write_text(text)
