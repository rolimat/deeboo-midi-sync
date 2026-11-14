# Reenvío del clock con marcas de tiempo (CoreMIDI)

Fecha: 2026-11-14. Amplía el diseño base
[`2026-11-05-deeboo-midi-sync-design.md`](2026-11-05-deeboo-midi-sync-design.md) y sustituye lo que
este dice sobre marcas de tiempo (§5.3 pulso extra, §5.4 timestamps, §7 midir, §10.1 preferencia de
no bajar a CoreMIDI). El usuario delegó todas las decisiones de este cambio.

## 1. Por qué

Síntoma (usuario): con el master a 125 BPM, Deeboo muestra 125.0 fijo pero el tempo de Ableton
oscila 124.8–125.2. Con Ableton conectado directamente a la salida virtual del master, el tempo
de Ableton es exacto.

Mediciones del 2026-11-14 con una sonda CoreMIDI propia (Swift, marcas de paquete y hora de llegada):

| Punto | Desviación entre pulsos | Tempo pulso a pulso |
|---|---|---|
| Marca de tiempo del master | 0.63 µs | 124.99–125.01 |
| Hora de llegada a `Deeboo Clock Sync In` | 22 µs | 124.56–126.06 |
| Salida de Deeboo (marca 0, Ableton usa la llegada) | 36.6 µs (p99 96 µs) | 124.23–125.72 |

- El master marca cada pulso con su hora exacta: un pulso por paquete, siempre en el pasado
  (llega 21–226 µs después de su marca, nunca antes), intervalos de 19 999–20 001 µs.
- Deeboo ignora la marca de entrada y `midir` envía siempre con marca 0 ("ahora"); su opción
  `coremidi_send_timestamped` pone la hora actual. `midir` no permite enviar con una marca elegida.
- Ableton usa las marcas de los paquetes: la llegada desde el master ya oscila 124.56–126.06, y aun
  así Ableton directo es exacto. Con marca 0 solo tiene la llegada y el número baila.
- La salida virtual del master es una fuente virtual que hace exactamente lo que hará Deeboo
  (`MIDIReceived` con marcas ya en el pasado al entregarse), y Ableton la sigue sin oscilar. Por eso
  no hace falta un experimento previo para saber si Ableton respeta marcas pasadas de una fuente
  virtual.
- El pulso extra del nudge (Push) sale por el hilo temporizador con hasta ±1 ms de error respecto a
  la mitad del intervalo (intervalos medidos 8 978–10 970 µs en vez de 10 000).

## 2. Objetivos, no objetivos y anti-objetivos

Objetivos:
- Cada pulso reenviado sale con la marca de tiempo con que llegó (precisión ≤ 1 µs por la
  conversión a µs). Medido en `Deeboo Clock Sync Out`: desviación entre pulsos en reposo ≤ 2 µs.
- El pulso extra del Push sale marcado exactamente en `marca del pulso anterior + intervalo/2`,
  aunque el temporizador despierte tarde.
- Las marcas de salida nunca retroceden y nunca están en el futuro.
- Ableton (el esclavo del usuario) muestra el tempo exacto a través de Deeboo, como en directo.

No objetivos:
- Masters que marquen por adelantado (marca en el futuro): se usa "ahora", igual que hoy.
- Cambiar el algoritmo del nudge, las reglas de transporte, la UI, los ajustes o los nombres de puerto.
- Soporte de Linux o Windows (la app es solo macOS).
- Pasar la sonda de jitter a CoreMIDI directo: su entrada `midir` ya lee la marca del paquete, así que
  mide lo que ve el esclavo.

Anti-objetivos (fallo aunque "funcione"):
- Marcas fuera de orden en la salida, o un `Clock` marcado antes de su `Start`.
- Un cuelgue o pánico por marcas raras de otra app.
- Perder el chequeo en Docker del resto del código.

## 3. Restricciones

- `clock-core` sigue puro: sin E/S, sin reloj del sistema; el tiempo llega como argumento.
- El crate `coremidi` 0.9.2 ya está en `Cargo.lock` (lo usa `midir`); no entra código de terceros nuevo.
- `coremidi` solo compila en macOS; el chequeo `just precommit` corre en Docker Linux.
- TDD obligatorio (AGENTS.md).

## 4. Enfoques

| Enfoque | Qué | Veredicto |
|---|---|---|
| A. Híbrido | Entrada con `midir`, salida con `coremidi` | Descartado: dos librerías MIDI en la app y reconversión µs→ticks de las marcas de `midir` |
| **B. Solo `coremidi` en la app** | Entrada y salida con `coremidi`, marcas nativas de extremo a extremo | **Elegido** |
| C. Mejorar `midir` | Hilo de más prioridad, enviar antes | Descartado: la variación nace en la entrega de macOS, antes de nuestro código, y `midir` no puede marcar |
| D. Programar el pulso extra con marca futura | Enviarlo junto al pulso anterior, marcado a mitad de intervalo | Descartado: semántica incierta de marcas futuras en fuentes virtuales, y un Pause posterior no podría retirarlo |

B porque la app es solo macOS (la portabilidad de `midir` no aporta), expone las marcas y paquetes
tal cual, y el cambio queda dentro del puente. Coste: el código MIDI pasa a ser solo macOS; lo
compensa el job macOS de CI, que ya ejecuta clippy y tests de todo el workspace.

## 5. Diseño

### 5.1 Base de tiempo

- Un único reloj: el host time de CoreMIDI (`mach_absolute_time`) en µs. Sustituye a `MonoClock`
  (`Instant`). Las marcas de paquete de CoreMIDI están en esa misma base, así que entrada, motor y
  salida comparten reloj.
- Conversión ticks↔µs con `mach_timebase_info` (`numer/denom`; 125/3 en Apple Silicon, 1/1 en
  Intel), en aritmética `u128` sin desbordes. La conversión es una función pura (`Timebase`), con
  tests que corren en Docker. Solo la lectura de `mach_absolute_time`/`mach_timebase_info` es
  específica de macOS (FFI mínimo a libSystem); fuera de macOS el reloj usa `Instant` para que el
  stub compile.

### 5.2 Entrada

- Puerto `Deeboo Clock Sync In`: destino virtual de `coremidi` con la API `PacketList`
  (MIDI 1.0, bytes tal cual; la API UMP obligaría a decodificar palabras).
- Cada paquete de la lista es una llamada `on_midi(at_us, datos)`, con `at_us` = hora del evento.
  Todos los bytes de un paquete comparten su marca (un paquete `F8 F8` cuenta dos pulsos con la misma
  hora, como hoy; el medidor ignora el intervalo 0).
- Validación de la marca (frontera de confianza, viene de otra app), función pura testeada:
  - marca 0 → ahora;
  - marca en el futuro → ahora;
  - marca más antigua que `SIGNAL_TIMEOUT_US` (500 ms) → ahora (marca basura o de otra base).

### 5.3 Motor (`clock-core`)

- `Output` lleva una marca por byte: `Output { send: Vec<TimedByte> }` con
  `TimedByte { at_us: u64, byte: u8 }`, más `Output::bytes() -> Vec<u8>` para quien solo mire bytes.
- Marca de cada salida:
  - `0xF8` reenviado y `0xFC` entrante reenviado: el `at_us` de `on_midi`;
  - pulso extra del Push: su hora programada (`pending_extra_us`), no la del tick que lo dispara;
  - `Start`/`Stop` de Play/Pause: el `now_us` del comando;
  - `Stop` por pérdida de señal: el `now_us` del tick.
- Invariante: las marcas de salida no decrecen. El motor recuerda la última marca emitida y sube
  cualquier marca menor hasta ella. Un pulso marcado antes del Play (ocurrió antes de pulsarlo) no se
  reenvía; el esclavo arranca con el siguiente.
- `on_midi(at_us, bytes)` conserva su firma: `at_us` es la única hora que usa, también para el
  `on_tick` interno que vacía el pulso extra pendiente. La pérdida de señal la sigue vigilando el hilo
  temporizador con la hora real. `at_us` puede ser algo anterior al último `on_tick(now)` (las marcas
  llegan ~0.1 ms tarde); el motor ya lo tolera (restas saturadas, el medidor ignora intervalos ≤ 0) y
  los tests lo cubren.
- El campo `send` cambia de tipo y se migran todos sus usos (motor, tests, puente). Solo `src-tauri`
  usa `clock-core`; la sonda de jitter no.
- El BPM se mide con las marcas del master, así que también es más preciso.

### 5.4 Salida

- Puerto `Deeboo Clock Sync Out`: fuente virtual de `coremidi`.
- Cada `Output` se convierte en un `PacketBuffer` con un paquete por byte y su marca en ticks, y se
  envía con una sola llamada `received` (`MIDIReceived`), aún bajo el lock del motor (orden de lock
  motor → salida, sin cambios). Un error se registra con `log::warn!`.

### 5.5 Cliente y puertos

- Un `coremidi::Client` (`Deeboo MIDI Sync`) para toda la vida de la app, creado en
  `Bridge::start`. Si falla, ambos puertos quedan "no ok" con `log::error!`; nunca pánico.
- `rename_ports` suelta destino y fuente y los recrea con los nuevos nombres; el motor no cambia.
- Fuera de macOS el puente es un stub: los puertos nunca se abren (como hoy en `cfg(not(unix))`).

### 5.6 Dependencias, CI y herramientas

- `src-tauri`: se quita `midir`; `coremidi = "0.9"` como dependencia solo de
  `cfg(target_os = "macos")`.
- `tools/jitter-probe` sigue con `midir` (y con él `libasound2-dev` en Docker).
- Docker compila el stub; el job macOS de CI (clippy `-D warnings` + `cargo test --workspace`)
  cubre el código CoreMIDI. Nueva receta de host `just check-macos` (clippy + tests en macOS) para
  ejecutarlo en local; se documenta en el skill `dev-workflow`.
- Test de integración en macOS con CoreMIDI real: el puente se arranca con nombres de puerto únicos,
  un cliente de test envía pulsos marcados cada 20 000 µs al destino, se pulsa Play y se capturan los
  paquetes de la fuente. Se comprueba que las marcas de salida coinciden con las de entrada (±1 µs) y
  que, con Push, el pulso extra cae en `anterior + 10 000` µs (±1 µs). Si el runner de CI no puede
  crear clientes MIDI, el test pasa a `#[ignore]` y se ejecuta con `just check-macos`.

### 5.7 Documentación

- AGENTS.md: arquitectura (coremidi en la app, midir solo en la sonda) y la regla de oro del reloj
  ("un reloj: host time de CoreMIDI en µs; la marca del paquete es la hora del evento y se conserva
  al reenviar").
- Skills `rust-engineer` (refs `deeboo-tauri-app`, `deeboo-clock-core`), `midi-clock-domain`
  (ref `transport-and-nudge`), `writing-tests` (ref `rust-simulated-time`), `dev-workflow`.
- Diseño base: §5.3, §5.4, §7 y §10.1 remiten a este documento.

## 6. Verificación

- Tests de `clock-core` (tiempo simulado): marcas de reenvío, pulso extra marcado a su hora aunque el
  tick llegue tarde, marcas monotónicas (Start antes de un pulso con marca anterior), `on_midi` con
  hora anterior al último tick, y los tests existentes adaptados a `bytes()`.
- Tests puros del puente: ida y vuelta ticks↔µs con varios timebases y la validación de marcas.
- Test de integración CoreMIDI (§5.6) en macOS.
- En el Mac del usuario, tras instalar el DMG (comprobación manual, no bloquea la release):
  `just jitter --seconds 30 --connect "Deeboo Clock Sync Out"` con Deeboo en Play debe dar desviación
  ≤ 2 µs en reposo, y el usuario confirma el tempo estable en Ableton.

## 7. Decisiones del diseñador (el usuario las delegó)

- Enfoque B y su coste en CI (§4, §5.6).
- Límite de antigüedad de marca = `SIGNAL_TIMEOUT_US`.
- Monotonía forzada en el motor, no en el puente, para poder testearla con tiempo simulado.
- Precisión de 1 µs (el motor trabaja en µs enteros), suficiente frente a la granularidad del master.
- La sonda de jitter no cambia.
- Versión de la release: 1.1.0, con la subida de versión como última tarea del plan; el usuario pidió
  explícitamente PR, merge a `main` y tag.
- Antes del tag se mergea la PR #6 (nombre del DMG en `SHA256SUMS`), ya verde, para que la release
  publique sumas correctas.

## 8. Dejado al constructor

- Nombres exactos de tipos y funciones nuevas, y dónde vive el FFI de host time.
- Estructura del código específico de macOS dentro de `src-tauri` (módulo propio o `cfg` en `bridge.rs`).
