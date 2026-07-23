<div align="center">

# ⚡ EnergiBridge

**One command-line tool to measure software energy consumption — across every major OS and CPU.**

[![Release](https://github.com/tdurieux/EnergiBridge/actions/workflows/release.yml/badge.svg)](https://github.com/tdurieux/EnergiBridge/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/tdurieux/EnergiBridge)](https://github.com/tdurieux/EnergiBridge/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Paper](https://img.shields.io/badge/paper-arXiv%3A2312.13897-e5503a)](https://arxiv.org/abs/2312.13897)

[Install](#install) · [Usage](#usage) · [Output](#output) · [Citation](#citation)

</div>

## What it does

Measuring how much energy software uses is awkward: every operating system and CPU exposes different, low-level counters, so experiments are hard to set up and even harder to reproduce on another machine.

EnergiBridge wraps **any command** and records power, frequency, temperature, CPU/GPU usage, and memory at fixed intervals to a **CSV** — behind a single interface, so your energy experiments run and reproduce the same way everywhere.

### Platform support

| OS      | Intel CPU | AMD CPU | M-series | Intel GPU | Nvidia GPU | AMD GPU | M-series GPU |
| ------- | :-------: | :-----: | :------: | :-------: | :--------: | :-----: | :----------: |
| Linux   |    ✅     |   ✅    |          |           |     ✅     |         |              |
| Windows |    ✅     |   ✅    |          |           |     ✅     |         |              |
| macOS   |    ✅     |         |    ✅    |    ✅     |            |   ✅    |      ✅      |

## Install

Prebuilt binaries are available on the [releases page](https://github.com/tdurieux/EnergiBridge/releases). To build from source you need a [Rust toolchain](https://rustup.rs/); NVIDIA GPU support additionally requires `nvml`.

<details>
<summary><b>Linux</b></summary>

Grant read access to the MSR files (reset on every reboot):

```bash
sudo chgrp -R msr /dev/cpu/*/msr
sudo chmod g+r /dev/cpu/*/msr
```

Build, then grant the binary the `rawio` capability (re-run if you move the binary):

```bash
cargo build -r
sudo setcap cap_sys_rawio=ep target/release/energibridge
```

</details>

<details>
<summary><b>Windows</b></summary>

EnergiBridge uses LibreHardwareMonitor to read the CPU registry. In an **elevated** command prompt (use `sc.exe` in PowerShell):

```bat
sc create rapl type=kernel binPath="<absolute_path_to_LibreHardwareMonitor.sys>"
sc start rapl
cargo build -r
```

Manage the driver with `sc stop rapl` / `sc delete rapl`.

</details>

<details>
<summary><b>macOS</b></summary>

```bash
cargo build -r
```

</details>

## Usage

```
energibridge [OPTIONS] [COMMAND]...

Options:
  -o, --output <OUTPUT>            Write measurements to this CSV file
  -s, --separator <SEPARATOR>     CSV separator [default: ,]
  -c, --command-output <FILE>     Capture the command's own output
  -i, --interval <INTERVAL>       Milliseconds between measurements [default: 200]
  -m, --max-execution <SECONDS>   Cap the command duration (-1 to disable) [default: 0]
  -g, --gpu                       Also collect GPU usage
      --summary                   Print total energy consumption at the end
  -h, --help                      Print help
  -V, --version                   Print version
```

Example — measure a build and print a summary:

```bash
energibridge --summary -o results.csv -- cargo build --release
```

## Output

EnergiBridge writes one CSV row per interval. Units:

| Time | Energy | Memory | Frequency | Voltage |
| :--: | :----: | :----: | :-------: | :-----: |
|  ms  |   J    | Bytes  |    MHz    |    V    |

<details>
<summary>Sample CSV</summary>

```csv
Delta,Time,CPU_FREQUENCY_0,...,CPU_TEMP_0,...,CPU_USAGE_0,...,SYSTEM_POWER (Watts),TOTAL_MEMORY,TOTAL_SWAP,USED_MEMORY,USED_SWAP
0,1697704464320,0,...,46.52,...,46.37,...,11.58,34359738368,0,10188488704,0
104,1697704464321,0,...,46.52,...,46.37,...,11.58,34359738368,0,10189275136,0
```

</details>

## Citation

If you use EnergiBridge in your research, please cite it (see [`CITATION.cff`](CITATION.cff)):

> J. Sallou, L. Cruz, T. Durieux. *EnergiBridge: Empowering Software Sustainability through Cross-Platform Energy Measurement.* arXiv:2312.13897, 2023. <https://arxiv.org/abs/2312.13897>

## License

[MIT](LICENSE) © [June Sallou](https://orcid.org/0000-0003-2230-9351), [Luís Cruz](https://luiscruz.github.io/), and [Thomas Durieux](https://durieux.me)
