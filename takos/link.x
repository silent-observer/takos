OUTPUT_FORMAT("elf64-x86-64", "elf64-x86-64", "elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

SECTIONS
{
    . = 0xFFFFFFFF80000000 ;

    .text           :
    {
        *(.text.unlikely .text.*_unlikely .text.unlikely.*)
        *(.text.exit .text.exit.*)
        *(.text.startup .text.startup.*)
        *(.text.hot .text.hot.*)
        *(SORT(.text.sorted.*))
        *(.text .stub .text.* .gnu.linkonce.t.*)
        /* .gnu.warning sections are handled specially by elf.em.  */
        *(.gnu.warning)
    }

    . = 0xFFFFFFFFC0000000 ;

    .rodata : { *(.rodata) *(.rodata.*) }
    .eh_frame_hdr : { *(.eh_frame_hdr) }
    .eh_frame : { *(.eh_frame) }
    .data.rel.ro : { *(.data.rel.ro) *(.data.rel.ro.*) }
    .got : { *(.got) }
    .data : { *(.data) *(.data.*) }
    .bss : { *(.bss) *(.bss.*) }
}