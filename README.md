# 6502-computer-emulator
A custom 6502 based computer, emulated in Rust

to run, use `cargo run -- [input file]` or compile the project using `cargo build --release`, the result file will be saved in the `target/release` folder

# Command line arguments
- `--ticks` or `-t`
  - Sets how many clock cycles per frame will be processed. Default is 1.
  - **Usage**: --ticks [ticks]
- `--cartridge` or `-c`
  - Loads a ROM accessible through the interface adapter. Maximum addressable ROM size is 2^24 bytes.
  - **Usage**: --cartridge [file]
- `--delay` or `-d`
  - Waits a certain amount of time after each frame. Default is 0.
  - **Usage**: --delay [amount]
- `--update-each-changed`
  - Updates the screen on the given amount of framebuffer changes. Default is 1.
  - **Usage**: --update-each-changed [changes]
- `--update-each`
  - Forces a screen update on the given amount of CPU cycles. Default is 3600.
  - **Usage**: --update-each [cycles]
