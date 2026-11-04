# Quickr: A pure P2P screen sharing solution written in Rust

> [!Caution]
> **Misuse Disclaimer:** <br>
> Quickr does not condone or support any unethical or illegal use of this software. Misuse, such as streaming of copyrighted material or invasion of privacy, is strictly against the Quickr guidelines. The author is not responsible for any misuse of the application.



## To execute, run on CMD: 
### For sender/streamer: 
`cargo run -- send > ..\outputs\sender1.out 2>&1`
### For client/receiver: 
`cargo run -- receive > ..\outputs\receiver1.out 2>&1`


## Dependencies (early draft, needs checking and updating):
- Windows 10-11 Operating System
- CMake
- C++ compiler libraries
- Rustup

## Raw Steps to build

- Prepare your Rust development env and C++ build env
- Run `winget install Kitware.CMake`on the command prompt
- run `cd gui`
- run `cargo run`
