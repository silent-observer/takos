# TakOS
This is a simple operating system written in Rust. It isn't complete yet, so don't expect much.
Mostly just an educational project to study how operating systems work inside.
Tested using QEMU and real hardware (as a live USB).

## Features
- [X] Bootloader
  - [X] UEFI bootloader, loads successfully from refind
  - [X] Loads kernel from an ELF file in the filesystem
  - [X] Simple memory allocator
  - [X] Configures initial memory map
  - [X] Also loads ramdisk
- [X] Basic hardware setup
- [X] Hardware interrupt and exception support
- [X] Hardware timers
  - [X] PIC
  - [ ] APIC
  - [ ] PIT
- [X] VGA support
- [X] Custom text rendering
- [X] Console output
- [X] Keyboard support (PS/2)
- [X] Console input support
- [X] Console scrolling
- [X] Paging memory allocator
- [X] Async/await implementation
  - [X] Async API for timers
  - [X] Async keyboard driver
- [X] PCI device enumeration 
- [X] FAT filesystem support (from ramdisk)
- [X] Simple file API
- [ ] Scheduling and multithreading
  - Partially implemented, but doesn't properly work yet
- [ ] USB support
  - Partially implemented (XHCI driver), works on QEMU emulation, doesn't work on real hardware for unknown reasons
