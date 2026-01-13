#!/bin/bash
qemu-system-x86_64 \
  -drive format=raw,file=rustos.img,index=0,media=disk \
  -serial stdio \
  -display cocoa \
  -m 512M \
  -cpu qemu64 \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04
