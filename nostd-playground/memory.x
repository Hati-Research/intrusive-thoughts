MEMORY {
    FLASH : ORIGIN = 0x00000000, LENGTH = 1024K
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

__sram3_start: ORIGIN(RAM) + 0x2000;
__sram3_end: sram3_start + 0x2000;

