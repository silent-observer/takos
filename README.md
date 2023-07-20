# TakOS - Simple operating system written in Rust

## Memory map

```
                     _____________________
FFFF FFFF FFFF FFFF |                     |
                    |     Kernel Stack    |  1024 KB
FFFF FFFF FFF0 0000 |_____________________|
FFFF FFFF FFEF FFFF |                     |
                    |       MMIO          |
FFFF FFFF FF00 0000 |_____________________|
FFFF FFFF FEFF FFFF |                     |
                    |     Kernel Data     |
FFFF FFFF C000 0000 |_____________________|
FFFF FFFF BFFF FFFF |                     |
                    |     Kernel Code     |
FFFF FFFF 8000 0000 |_____________________|
FFFF FFFF 7FEF FFFF |                     |
                    |       MMIO          |
FFFF F000 0000 0000 |_____________________|
FFFF EFFF FFFF FFFF |                     |
                    |     Kernel Heap     |
FFFF D000 0000 0000 |_____________________|
FFFF CFFF FFFF FFFF |                     |
                    | Physical Memory Map |
FFFF C000 0000 0000 |_____________________|
                     _____________________
FFFF BFFF FFFF FFFF |                     |
                    |   Userspace Stack   |
FFFF 8000 0000 0000 |_____________________|
        ...          _____________________
0000 7FFF FFFF FFFF |                     |
                    |   Userspace Heap    |
0000 4000 0000 0000 |_____________________|
0000 3FFF FFFF FFFF |                     |
                    |    Userspace Data   |
0000 3000 0000 0000 |_____________________|
0000 2FFF FFFF FFFF |                     |
                    |   Userspace Code    |
0000 2000 0000 0000 |_____________________|
0000 1FFF FFFF FFFF |                     |
                    |      Reserved       |
0000 0000 0000 0000 |_____________________|


```