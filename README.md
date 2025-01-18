# UdpForward

Some application may listen to all interfaces by binding UDP socket to 0.0.0.0

However the same socket may be reused to reply back. With multiple interfaces, windows may choose any of the interfaces and this can break communications.

This app will attempt to bind to single interface and forward the packet.

## Compiling
1. Download rust
2. Run `cargo build --release`
