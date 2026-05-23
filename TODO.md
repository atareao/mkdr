# TODO

## Renderizado

- [x] **Strikethrough** — ~~texto~~ renderiza con estilo `strikeout` (#808080 + crossed out)
- [ ] **Task lists** — `- [x]` / `- [ ]` con íconos ☑/☐ y estilo
- [ ] **Tablas: alineación** — respetar alineación izquierda/derecha/centro
- [ ] **Imágenes** — mostrar `![alt](url)` como link con ícono
- [ ] **Enlaces por referencia** — `[text][ref]` + `[ref]: url`

## Navegación

- [ ] **Historial de búsqueda** — `↑`/`↓` recupera búsquedas anteriores
- [ ] **Marcadores** — `m{a-z}` marca línea, `'{a-z}` salta a marca
- [ ] **Ir a porcentaje** — `:50%` salta al 50% del documento

## UX

- [x] **Auto-crear `~/.config/mdr/themes/`** si no existe al arrancar
- [x] **Completado de shell** — `mdr --completions bash`
- [ ] **Pipe status bar** — mostrar "stdin" en barra al leer de pipe
- [ ] **Múltiples archivos: lista** — `:files` muestra índice navegable
- [ ] **Temas desde TUI** — `:theme nord` cambia sin reiniciar
- [ ] **Recargar configuración** — `:reload` recarga config + theme

## Testing

- [ ] **Tests de integración** — en `tests/` para la app completa
- [ ] **Tests de snapshot: más casos** — listas anidadas, blockquote anidado, tablas complejas
- [ ] **Tests de search** — búsqueda + highlight sobre contenido renderizado

## Empaquetado

- [ ] **Completado bash/zsh/fish** en CI (generar y subir en release)
- [ ] **Homebrew formula** — `brew install atareao/tap/mdr`
- [ ] **AUR package** — `PKGBUILD` para Arch Linux
- [ ] **cargo-binstall** — soporte para instalar desde binarios

## Bugs conocidos

- [ ] **Wrap + tablas** — con `--wrap word`, las tablas se rompen
- [ ] **Archivo no encontrado** — mensaje de error mejorado (sugerir `--help`)