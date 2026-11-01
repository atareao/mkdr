# Tasks

## 1. RED — tests que deben fallar primero

- [x] 1.1 `src/render.rs`: ampliar el test de supresión de estilos para que su fixture incluya código inline, enlace, tabla y tachado, y para que exija modificadores vacíos además de color de primer plano y de fondo nulos.
- [x] 1.2 `src/render.rs`: test de que con la política activa los títulos, la negrita, la cursiva, el tachado, el código inline, los enlaces, las viñetas, las marcas de lista numerada, la marca de cita, la regla horizontal y los checkboxes de task list quedan sin estilo.
- [x] 1.3 `src/render.rs`: test de regresión de que sin la política los mismos elementos vuelven a llevar estilo.
- [x] 1.4 Búsqueda: la cobertura RED llegó a nivel de frame desde `src/app.rs` (test de frame con la búsqueda activa, la coincidencia localizada y la barra de estado informando de la posición). El test unitario de `src/search.rs` —con la política activa el resaltado no aplica estilo y sigue partiendo los spans en las coincidencias; sin la política mantiene el resaltado— se añadió en GREEN.
- [x] 1.5 `src/app.rs`: tests de que la barra de estado, la lista de ficheros, la numeración de líneas y el estilo del frontmatter en la vista raw quedan sin estilo con la política activa.
- [x] 1.6 `src/app.rs`: test de frame completo con backend de pruebas en memoria sobre un documento con todas las construcciones, exigiendo estilo por defecto en todas las celdas del buffer; repetirlo con el modo búsqueda activo y con el modo lista de ficheros abierto.
- [x] 1.7 Ejecutar `cargo test` y confirmar que los tests nuevos fallan y que los 73 tests existentes siguen en verde.

## 2. GREEN — implementación mínima

- [x] 2.1 `src/app.rs`: recibir la política de supresión de estilos como parámetro explícito y conservarla como estado; eliminar la deducción a partir de la ausencia de estilos en el tema.
- [x] 2.2 `src/main.rs`: pasar la política solicitada por la línea de comandos a la construcción de la aplicación.
- [x] 2.3 `src/render.rs`: ayudante único que resuelva a estilo por defecto con la política activa; aplicarlo a los fallbacks de la construcción del renderer y a los colores literales de los puntos de uso (viñetas, listas, cita, reglas, checkboxes, código inline, enlaces, bordes de tabla, títulos).
- [x] 2.4 `src/app.rs`: aplicarlo a la barra de estado, la lista de ficheros, la numeración de líneas y el frontmatter de la vista raw.
- [x] 2.5 `src/search.rs`: recibir la política en el resaltado de coincidencias y no aplicar estilo cuando esté activa; actualizar las llamadas.
- [x] 2.6 Ejecutar `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo fmt --all -- --check`; los tres deben salir limpios.

## 3. REFACTOR

- [x] 3.1 Eliminar duplicación entre los ayudantes de estilo sin cambiar comportamiento.
- [x] 3.2 Revisar con una búsqueda en el código que no quedan literales de color ni de modificador fuera de los ayudantes y de los puntos donde la política los permite.
- [x] 3.3 Volver a ejecutar la batería completa de tests y el linter tras el refactor.

## 3bis. Correcciones de la revisión independiente

- [x] 1.1 Añadir en `src/render.rs` el test de regresión `render_plain_theme_without_no_colour_keeps_behaviour`, que cubre el escenario «tema sin estilos y sin el flag»: con `Theme::plain()` y `no_colour = false` el texto renderizado es idéntico al de la política activa y los fallbacks del renderer (negrita incluida) siguen aplicándose. El test falla si la supresión se aplicara con el flag inactivo.
- [x] 1.2 Corregir en `design.md` (decisión 4) la afirmación inexacta del criterio: no es «cero secuencias SGR con el flag, más de cero sin él», sino cero secuencias SGR **que apliquen color o modificador** con el flag frente a del orden de varias decenas sin él (≈73 en la medición de referencia, según documento y geometría del terminal), excluyendo los cuatro reinicios neutros de teardown del pty (`ESC[39m ESC[49m ESC[59m ESC[0m`). Aplicado el mismo criterio a la redacción del requisito y del escenario de medición de `spec.md`, sin cambiar requisitos ni escenarios.
- [x] 1.3 Endurecer `prepend_line_number` en `src/app.rs`: recibe el estilo ya resuelto y el llamante pasa `self.chrome_style(Style::default().fg(Color::DarkGray))`, eliminando la fuga latente. Actualizados los tests `prepend_line_number_adds_span` y `prepend_line_number_pads_width` (aserción de estilo incluida) y añadido `prepend_line_number_is_plain_under_no_colour_policy`.
- [x] 1.4 Ampliar la cobertura de frame del chrome bajo la política en `src/app.rs`: aserciones de buffer plano para los prompts de `Mode::SearchForward` / `Mode::SearchBackward` / `Mode::GoToLine` y para el span del mensaje de estado (`⚠`), reutilizando el backend de pruebas y los ayudantes `cell_is_plain` / `buffer_is_plain` mediante ayudantes internos compartidos.
- [x] 1.5 Eliminar el método `Theme::is_plain()` (y su `#[allow(dead_code)]`), adaptando `plain_theme_has_no_styles`, `default_dark_is_not_plain` y `default_light_is_not_plain` para asertar el comportamiento subyacente de `style_for(...)`.

## 4. Verificación de aceptación

- [x] 4.1 Compilar en modo release.
- [x] 4.2 Ejecutar en un pty el binario con la política activa sobre un documento con todas las construcciones y comprobar, sobre la salida cruda del pty, que no se emite ninguna secuencia SGR que aplique color o modificador. Los cuatro reinicios neutros del teardown (`ESC[39m ESC[49m ESC[59m ESC[0m`), que se emiten con y sin el flag, no cuentan.
- [x] 4.3 Ejecutar el mismo documento sin la política y comprobar que los estilos siguen presentes: el recuento de secuencias SGR **que aplican color o modificador** en la salida cruda del pty se mantiene en el orden de la versión anterior al cambio (del orden de varias decenas, ≈73 en la medición de referencia, según documento y geometría del terminal), como comprobación de no regresión.
- [x] 4.4 Comprobar en un pty que la búsqueda sigue localizando coincidencias con la política activa y que la barra de estado informa de la posición.
- [x] 4.5 Validar el change con la herramienta de OpenSpec y marcar las tareas completadas.

## 5. Consolidación

- [ ] 5.1 Abrir pull request de la rama del change hacia la rama de integración siguiendo el flujo del repositorio.
- [ ] 5.2 Archivar el change con la herramienta de OpenSpec una vez integrado, para volcar los requisitos a las especificaciones del proyecto.
