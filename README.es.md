<p align="center">
  <img src="docs/logo.png" alt="InkReset" width="96" />
</p>

<h1 align="center">InkReset</h1>

<p align="center">
  <a href="https://github.com/datacas/inkreset/releases/latest"><strong>Descargar la última versión para Windows</strong></a>
</p>

<p align="center">
  <a href="https://github.com/datacas/inkreset/releases/latest"><img alt="Última versión" src="https://img.shields.io/github/v/release/datacas/inkreset?style=flat-square&color=e2542f"></a>
  <a href="https://github.com/datacas/inkreset/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/datacas/inkreset/ci.yml?style=flat-square"></a>
  <img alt="Descargas" src="https://img.shields.io/github/downloads/datacas/inkreset/total?style=flat-square">
  <img alt="Licencia" src="https://img.shields.io/github/license/datacas/inkreset?style=flat-square">
  <img alt="Plataforma" src="https://img.shields.io/badge/platform-windows%20x64-2b6cb0?style=flat-square">
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>Español</strong>
</p>

InkReset es una utilidad de escritorio de código abierto que lee y reinicia los
**contadores de tinta residual** de impresoras Epson por USB. Habla directamente con
la impresora mediante IEEE 1284.4 y los comandos de servicio de la EEPROM de Epson:
sin cuenta del fabricante, sin clave de licencia, sin servidor de activación y sin
acceso a la red.

> **Reiniciar un contador no es mantenimiento.**
> Reiniciar el contador **no** limpia, sustituye ni vacía el absorbente físico de tinta
> residual. Atiende primero la almohadilla. Reiniciar una almohadilla saturada puede
> hacer que la tinta se desborde.

## Hardware probado

InkReset solo se ha probado de principio a fin en una **Epson XP-530**
(`MDL:XP-530 Series`): lectura de los dos contadores de residuos y prueba de escritura
segura. La base de datos integrada cubre muchos otros modelos Epson y el protocolo
debería funcionar en ellos, pero **esos modelos no están probados**.

Si usas otra impresora, haz primero una lectura y la prueba de escritura segura, y
reinicia bajo tu responsabilidad. Los autores no se hacen responsables de los daños
causados a otras impresoras.

---

## Funciones

- **Detección automática.** Al arrancar, InkReset enumera las interfaces de impresora
  USB, lee el identificador IEEE 1284 del dispositivo y resuelve el perfil
  correspondiente en la base de datos integrada.
- **Lecturas reales.** Se lee por USB cada contador de residuos que expone el modelo,
  con los valores de registro en bruto y el fondo de escala que declara la impresora.
- **Reinicio condicionado.** Solo se ejecuta si el modelo tiene una tabla de reinicio
  verificada, el hardware sigue informando del mismo `MDL:` y el usuario escribe ese
  modelo exactamente. Después se vuelven a leer los contadores para verificar la
  escritura.
- **Prueba de escritura segura.** Una autoprueba dedicada reescribe un byte de la
  EEPROM sin cambiarlo, demostrando que toda la ruta de escritura funciona sin alterar
  ningún dato.
- **Interfaz bilingüe.** Español por defecto, inglés disponible en Preferencias.
- **Solo local.** Los ajustes y el historial viven en `localStorage`. No hay telemetría,
  ni registro remoto, ni ninguna petición saliente.
- **Sondas sin interfaz.** El mismo núcleo nativo puede enumerar impresoras y leer
  contadores desde la línea de comandos, sin abrir la ventana.

## Capturas

![Banco de servicio de InkReset leyendo una Epson XP-530](docs/bench.png)

| Índice de modelos | Preferencias y puente de hardware |
| --- | --- |
| ![Índice de modelos](docs/models.png) | ![Preferencias](docs/settings.png) |

![Sobre esta compilación](docs/about.png)

## Tecnología

| Capa | Tecnología |
| --- | --- |
| Interfaz | React 19, TypeScript, Vite |
| Contenedor de escritorio | Tauri 2 |
| Puente nativo | Rust |
| Transporte USB | Windows SetupAPI / `USBPRINT` + `ReadFile` / `WriteFile` solapados |
| Protocolo de impresora | Sesión IEEE 1284.4 + comandos de servicio de la EEPROM de Epson |

La interfaz se dibuja dentro del WebView de Tauri. Cada escaneo, lectura y reinicio es
una transacción USB real ejecutada por el puente en Rust; la capa web solo describe la
intención.

## Cómo funciona el protocolo

InkReset implementa la sesión IEEE 1284.4 que usan las herramientas de servicio de
Epson:

1. Entrar en modo 1284.4.
2. Inicializar la sesión y resolver el nombre del servicio a un id de socket.
3. Abrir un canal con los tamaños de paquete que declara el perfil del dispositivo.
4. Intercambiar concesiones y peticiones de crédito y enviar un paquete de datos por
   cada byte de EEPROM.
5. Cerrar el canal y salir del modo 1284.4.

El comando de EEPROM que viaja dentro de un paquete de datos es:

```
lectura: 7C 7C <len8> 00 <factory LE16> 41 BE A0 <addr_lo> [addr_hi]
escritura: 7C 7C <len8> 00 <factory LE16> 42 BD 21 <addr_lo> [addr_hi] <valor>
```

La impresora responde a una lectura con `EE:<addr><value>` y a una escritura con `OK`.
Las respuestas de control llevan el byte de estado y, para las tramas `0x7F`, el id de
error de la impresora. Un canal ya abierto (`0x03`) se trata como éxito; un canal
ocupado (`0x04`) se reintenta un número acotado de veces.

Por modelo, una cadena de claves de especificación aporta el bloque de servicio
(token de fábrica y nombre de servicio), los contadores de residuos (`registers`,
`mask`, `shift`, `filter`) y la tabla de reinicio de pares `(dirección, valor)`. La
base de datos integrada incluye **1.022 registros de impresora** y **371 claves de
especificación**, de las cuales unas 930 resuelven a un bloque de servicio utilizable.

## Requisitos

- Windows 10 u 11 (el puente USB nativo es solo para Windows)
- Runtime de WebView2
- Node.js 20+
- Rust estable con la cadena de herramientas MSVC
- Visual Studio Build Tools con el Windows SDK

En Linux y macOS la interfaz compila y la aplicación arranca, pero el puente se declara
no disponible y no se contacta con ningún dispositivo.

## Compilar y ejecutar

```bash
npm install

npm run dev          # solo interfaz en un navegador (sin acceso USB)
npm run tauri:dev    # contenedor nativo con el puente USB real
npm run tauri:build  # ejecutable e instalador de release
```

Una compilación de release debe incorporar la interfaz. Tauri lo hace mediante la
característica `custom-protocol`:

```bash
cargo build --release --features custom-protocol
```

## Línea de comandos

```bash
inkreset-studio.exe --probe
inkreset-studio.exe --probe --read-counters
inkreset-studio.exe --self-test
inkreset-studio.exe --raw-session --sockets 0A,FF --names EPSON-CTRL,EPSON-DATA
```

| Opción | Para qué sirve |
| --- | --- |
| `--probe` | Enumera las interfaces USBPRINT y escribe `%TEMP%\inkreset-probe.json` |
| `--probe --read-counters` | Además ejecuta la sesión 1284.4 de solo lectura e incluye los contadores |
| `--self-test` | Reescribe un byte de la EEPROM sin cambiarlo e informa del resultado |
| `--raw-session` | Vuelca el handshake 1284.4 trama a trama; nunca escribe en la EEPROM |

Todos los comandos imprimen JSON por la salida estándar y escriben el mismo informe en
`%TEMP%`.

## Modelo de seguridad

- **Perfil verificado por el dispositivo.** El modelo se vuelve a leer del identificador
  IEEE 1284 de la impresora antes de escribir; el webview no puede proporcionarlo.
- **Confirmación escrita.** El usuario debe teclear la cadena `MDL:` exacta.
- **Sin tabla de reinicio no hay escritura.** Los modelos con bloque de servicio pero
  sin datos de reinicio son de solo lectura.
- **Verificación posterior.** Los contadores se leen de nuevo y se devuelven junto al
  resultado del reinicio.
- **Aviso permanente.** La interfaz y todos los informes indican que reiniciar un
  contador no atiende la almohadilla física.

## Estructura del proyecto

```
src/
├─ App.tsx                     # preferencias y proveedor de i18n
├─ Bench.tsx                   # estructura del banco de servicio
├─ components/                 # medidores, paneles, diálogos, índice de modelos
├─ hooks/usePrinterSession.ts  # estado del dispositivo e historial de sesión
├─ services/bridge/            # envoltorios de invoke() y PrinterTransport
├─ i18n/                       # diccionario tipado español/inglés
└─ styles/global.css           # sistema de diseño

src-tauri/
├─ src/protocol/mod.rs         # tramas 1284.4, códec de EEPROM, decodificación
├─ src/spec_db.rs              # base de datos de dispositivos integrada
├─ src/specs.json              # registros de impresora y claves de especificación
├─ src/usb/windows.rs          # enumeración SetupAPI y transporte solapado
├─ src/usb/ieee1284.rs         # analizador del identificador IEEE 1284
├─ src/commands.rs             # comandos de Tauri
└─ tauri.conf.json             # configuración de ventana y empaquetado
```

## Pruebas

El módulo de protocolo no tiene dependencias de terceros, así que se puede probar por
separado:

```bash
rustc --edition 2021 --test src-tauri/src/protocol/mod.rs -o /tmp/proto-test
/tmp/proto-test
```

En Windows, `cargo test` dentro de `src-tauri/` cubre además la base de datos de
dispositivos, el analizador del identificador IEEE 1284 y la sesión completa de
lectura y reinicio contra una impresora falsa con estado.

## Limitaciones

- **Solo USB.** Las impresoras conectadas por EpsonNet o WSD no aparecen.
- **Solo Epson.** La base de datos de dispositivos cubre modelos Epson.
- **Valores de registro en bruto.** Se aplican los campos `mask` y `shift`, pero no se
  reconstruye la fórmula de porcentaje que usan algunas entradas de filtro; la interfaz
  muestra el valor de registro en bruto y el fondo de escala de la impresora.
- **El índice de modelos es una lista de referencia curada**, no una garantía de que
  cada modelo listado exponga contadores legibles o reiniciables.
- **Solo la Epson XP-530 está verificada en hardware.** Otros modelos deberían
  funcionar, pero el usuario debe probarlos antes de reiniciar.

## Licencia

MIT — ver [LICENSE](LICENSE).

---

**Sustituye o atiende siempre el absorbente físico de tinta residual antes de
reiniciar un contador.**
