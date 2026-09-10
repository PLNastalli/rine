# tests/differential — vs Windows real (método)

Para cada PE: rodar no Windows real, salvar oracle
(`oracle/<nome>.{stdout,exit,json}`), rodar no `rine`, comparar byte-a-byte.
Runner (`compare.py`, v0.2) falhará em qualquer divergência. Windows é sempre
o oracle; divergência = bug + regression test (regra permanente).
