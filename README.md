# RustSBI implementation for HPMicro MCUs

[![Build](https://github.com/hpm-rs/rustsbi-hpm/actions/workflows/ci.yml/badge.svg)](https://github.com/hpm-rs/rustsbi-hpm/actions/workflows/ci.yml)

## 介绍

这是一个基于 [RustSBI](https://github.com/rustsbi/rustsbi)，用于 HPMicro MCUs 的 SBI 实现。支持以下功能：

### SEE (Supervisor Execution Environment)

目前支持以下 SBI 拓展：

- legacy console
- timer

### SDRAM 初始化

支持初始化 SDRAM 并映射到 AXI 总线。可用于后续内核的启动和执行。

### Linux 内核引导

支持引导 Linux 内核，并传递设备树。内核链接和烧录时请遵循如下布局。

| Name     | Base Address  | Load Address | Length    |
|----------|---------------|--------------|-----------|
| RustSBI  | 0x80003000    | 0x80003000   | 64 KB     |
| Kernel   | 0x40000000    | 0x80010000   | 3 MB      |
| DTB      | 0x40300000    | 0x80310000   | 16 KB     |

## 编译与烧录

通过如下命令生成烧录所需的 `.bin` 文件。

```shell
# 安装 cargo-binutils
cargo install cargo-binutils
# 生成 .bin 文件
cargo objcopy --release --features=flash  -- -O binary rustsbi.bin
```

编译完成后，可使用 [hpm_isp](https://github.com/tfx2001/hpm_isp) 进行烧录。修改启动模式选择管脚为 `BOOT_MODE[1:0]=0b10` 后将 USB0 连接至 PC，运行如下命令进行烧录。

```shell
hpm_isp flash 0 write 0x0 rustsbi.bin
```

## 支持的开发版

- [HPM6360EVK](http://hpmicro.com/resources/detail2.html?id=b60936f5-c3fe-4916-bb7d-854cc6bc5456)

## Rust 版本

```
rustc 1.81.0-nightly (6be96e386 2024-07-09)
```

# 相关链接

- [hpm-rs/buildroot](https://github.com/hpm-rs/buildroot) - 为 HPMicro MCUs 生成可启动的 Linux 镜像
