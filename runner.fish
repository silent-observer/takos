#!/bin/env fish
set INPUT (realpath $argv[1])
set DIR (cd (dirname (status -f)); and pwd)
cp $DIR/takobl/target/x86_64-unknown-uefi/release/takobl.efi $DIR/esp/efi/boot/bootx64.efi
cp $INPUT $DIR/esp/kernel.elf
qemu-system-x86_64 \
    -m 4G -s \
    -enable-kvm \
    -cpu host \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/qemu/ovmf-x86_64.bin \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/qemu/ovmf-x86_64-vars.bin \
    -device qemu-xhci,p2=8,p3=4,id=xhci \
    -drive id=pendrive,file=fat:rw:esp,format=raw,if=none \
    # -device usb-host,vendorid=0x0951,productid=0x1617,bus=xhci.0,port=5,pcap=usb.pcap \
    # -device usb-host,vendorid=0x046d,productid=0xc05a,bus=xhci.0,port=7 \
    # -device usb-host,vendorid=0x256c,productid=0x006d,bus=xhci.0,port=8 \
    -device usb-storage,drive=pendrive,pcap=usb.pcap,bus=xhci.0,port=6 \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    $argv[2..-1]
