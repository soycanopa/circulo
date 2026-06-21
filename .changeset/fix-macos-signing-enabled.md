---
"@circulo/desktop": patch
---

Corrige el code signing de macOS en CI: CSC_IDENTITY_AUTO_DISCOVERY vuelve a
"true" para que electron-builder use el CSC_LINK y firme+notarice los builds
de release. El 0.15.0 salió sin firma por este flag apagado y macOS mostraba
"Circulo is damaged and can't be opened".
