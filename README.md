# whisper-app

Real-time speech-to-text app using Whisper.cpp, built with Rust and Iced.

## WHY?
There are some UI implementations around whisper library, some of them seems to be offering cloud features.
Some of them seems to be very bulky, I wanted something minimal and functional, with performance and latency in focus.
And it should be simple enough to run in background without using AI features from Zoom or other providers.

## Demo
https://github.com/user-attachments/assets/f0a5140b-b326-41ce-a175-3f7c84aae73f
Almost live transcription on RTX 5060 TI 16 GB with medium model.

Smaller models are less accurate but are faster in CPU alone, your mileage may vary on your hardware.
## Quick Start

```bash
cd whisper-app
cargo run
```

## GPU Acceleration

By default, the app runs on CPU. Enable GPU acceleration at compile time:

### macOS — Metal

```bash
cargo run --features metal
```

### Linux — CUDA (NVIDIA)

```bash
cargo run --features cuda
```

### Linux — Vulkan (AMD/Intel)

```bash
cargo run --features vulkan
```

**Note:** GPU features are compile-time only — they cannot be toggled at runtime.

## Building for Release

```bash
cargo build --release --features cuda    # CUDA
cargo build --release --features metal   # Metal
```

## Prerequisites

- **Linux CUDA:** NVIDIA GPU, CUDA toolkit, and `libcudart.so` on `LD_LIBRARY_PATH`
- **Linux Vulkan:** `vulkan-loader` and GPU Vulkan driver (`mesa-vulkan-drivers` for AMD/Intel)
- **macOS Metal:** macOS 12+ (Apple Silicon or NVIDIA GPU with Metal support)
- **All platforms:** Rust toolchain (`cargo`)
