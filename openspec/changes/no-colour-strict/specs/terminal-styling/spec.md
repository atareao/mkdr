# Spec Delta

## Purpose

Definir cuándo `mkdr` aplica estilo a su salida y garantizar que, con la supresión de estilos activa, ningún carácter escrito en el terminal lleve color, énfasis ni cualquier otro atributo.

## ADDED Requirements

### Requirement: Supresión de estilos explícita

`mkdr` SHALL aplicar la supresión de estilos cuando y solo cuando se solicite explícitamente con `--no-colour` / `-c`, con independencia de que el tema activo tenga o no estilos definidos.

#### Scenario: Flag combinado con un tema con estilos

- **WHEN** se ejecuta `mkdr -t dracula --no-colour <fichero>`
- **THEN** la salida no contiene ningún atributo de estilo

#### Scenario: Tema sin estilos y sin el flag

- **WHEN** se renderiza un documento con un tema que no define estilos y sin `--no-colour`
- **THEN** se mantiene el comportamiento previo: los estilos por defecto (énfasis y colores de los fallbacks) siguen aplicándose

### Requirement: Documento sin estilos

Con la supresión activa, `mkdr` SHALL renderizar el documento de modo que ningún span lleve color de primer plano, color de fondo ni modificadores.

#### Scenario: Documento con todas las construcciones

- **WHEN** se renderiza con `--no-colour` un documento que contiene títulos, negrita, cursiva, tachado, código inline, bloque de código, enlace, tabla, lista de viñetas, lista numerada, cita, regla horizontal y task list
- **THEN** todos los spans de todas las líneas tienen color de primer plano nulo, color de fondo nulo y modificadores vacíos
- **AND** el texto de cada construcción se conserva íntegro, sin perder ni alterar caracteres

#### Scenario: Sin regresión cuando no se pide la supresión

- **WHEN** se renderiza el mismo documento sin `--no-colour`
- **THEN** los títulos vuelven a ir en negrita, la negrita y la cursiva se distinguen, el código inline vuelve a llevar su color y los enlaces su color y subrayado, y los bordes de tabla su color

### Requirement: Código sin color ni resaltado

Con la supresión activa, `mkdr` SHALL no aplicar color ni resaltado de sintaxis al código inline ni a los bloques de código.

#### Scenario: Bloque de código con lenguaje conocido

- **WHEN** se renderiza con `--no-colour` un documento con un bloque de código marcado con el lenguaje rust
- **THEN** ni el bloque ni el texto de código inline llevan color

### Requirement: Chrome de la TUI sin estilos

Con la supresión activa, `mkdr` SHALL renderizar su chrome —barra de estado, lista de ficheros, numeración de líneas y estilo del frontmatter en la vista raw— sin colores ni modificadores.

#### Scenario: Barra de estado

- **WHEN** se dibuja un frame con la supresión activa
- **THEN** ningún span de la barra de estado lleva color ni negrita

#### Scenario: Lista de ficheros

- **WHEN** se abre la lista de ficheros con la supresión activa
- **THEN** ningún span de la lista de ficheros lleva color ni negrita

#### Scenario: Numeración de líneas

- **WHEN** se dibuja con `--no-colour -n`
- **THEN** los números de línea no llevan color

### Requirement: Búsqueda sin estilos

Con la supresión activa, `mkdr` SHALL no resaltar las coincidencias de búsqueda con color ni modificadores, sin degradar la funcionalidad de búsqueda.

#### Scenario: Coincidencia encontrada sin resaltado

- **WHEN** se busca un término con la supresión activa
- **THEN** los spans que contienen la coincidencia no llevan color ni modificadores

#### Scenario: La búsqueda sigue siendo utilizable

- **WHEN** se busca un término que existe una sola vez en el documento con la supresión activa
- **THEN** la vista salta a la línea de la coincidencia
- **AND** la barra de estado informa del término y de la posición de la coincidencia

### Requirement: Garantía a nivel de salida

Con la supresión activa, `mkdr` SHALL dejar todas las celdas del área dibujada con el estilo por defecto, de modo que el flujo escrito al terminal no contenga secuencias SGR que apliquen color o modificador.

#### Scenario: Frame completo sin estilos

- **WHEN** se dibuja un frame con la supresión activa sobre un documento con todas las construcciones
- **THEN** todas las celdas del buffer dibujado tienen estilo por defecto

#### Scenario: Ejecución real en un terminal

- **WHEN** se ejecuta `mkdr --no-colour` en un terminal sobre ese mismo documento
- **THEN** el flujo escrito al terminal no contiene ninguna secuencia SGR que aplique color o modificador (los cuatro reinicios neutros del teardown —`ESC[39m ESC[49m ESC[59m ESC[0m`— no cuentan)
