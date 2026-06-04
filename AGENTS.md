# rustsbi-hpm

This document provides context for AI coding agents and developers working on the `rustsbi-hpm` project.

## Project Overview

RustSBI implementation for HPMicro MCUs. Provides a Supervisor Execution Environment (SEE) that boots Linux on RISC-V HPMicro MCUs.
**Core capabilities:** SBI timer/console extensions, SDRAM init, PMP configuration, and DTB/kernel loading from flash.

## Architecture & Constraints

- **Environment:** `#![no_std]`. This is bare-metal firmware. Do NOT use the Rust standard library (`std`). Do not assume heap allocation is available unless explicitly configured.
- **Target Architecture:** RISC-V 32-bit (Check `.cargo/config.toml` for the exact target, e.g., `riscv32imafc-unknown-none-elf`).
- **Toolchain:** Strictly follow the `rust-toolchain.toml` file.

## Build & Flash

### Build (Production / Flash mode)

```sh
cargo build --features=flash --release
cargo objcopy --features=flash --release -- -O binary rustsbi.bin
```

### Flash via USB0 (BOOT_MODE[1:0]=0b10)

```sh
hpm_isp flash 0 write 0x0 rustsbi.bin
```

## Features Configurations

Always use conditional compilation guards when modifying boot paths or memory layouts:

- `#[cfg(feature = "flash")]` — Booting from Quad SPI NOR Flash (production map).
- `#[cfg(feature = "ram")]` — Booting from RAM (debug map).

## Key Dependencies (Instructions for Agents)

When writing or modifying code, use the following crates instead of reinventing the wheel:

| Crate | Purpose | Agent Instruction |
|-------|---------|-------------------|
| `hpm-metapac` | Hardware register access | **Always** use this for PAC/register level access. Do not hardcode raw memory addresses for peripherals. |
| `hpm-rt` | Runtime init, cache, memory | Handles `build.rs` memory layout. Refer to this for linker script issues. |
| `rustsbi` / `sbi-spec`| SBI implementation | Adhere strictly to the SBI specification traits. |
| `fast-trap` | M-mode trap handling | Use this for RISC-V machine-mode exception/interrupt routing. |

## Memory Map & Payload (Linux/DTB)

*(Note: AI agents should refer to these physical addresses when configuring PMP or loading payloads)*

| Name     | Base Address  | Load Address | Length    |
|----------|---------------|--------------|-----------|
| RustSBI  | 0x80003000    | 0x80003000   | 64 KB     |
| Kernel   | 0x40000000    | 0x80010000   | 3 MB      |
| DTB      | 0x40300000    | 0x80310000   | 16 KB     |

## CI & Testing

- Only `cargo check` runs on push/PR to `main` or `dev/*`.
- **No automated tests exist currently.**
- When generating new code, ensure it compiles flawlessly for the `no_std` target architecture via `cargo check`.
