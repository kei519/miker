# Miker

A MICro KERnel OS for me to study.

## Requirements

- RAM:

    256 MiB - 512 GiB (maybe...)

### Requirements for Build

- Bash 4.2 or later

    We use `-v` option within double bracket to make an image, so Bash 4.2 or later and export PATH properly to use it.

## Building

For release build:

```
makers release
```

For debug build:

```
makers build
```

## Emulating on QEMU

Run

```
makers run <QEMU options>
```
