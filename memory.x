MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 32K
  /* The /401 version has 8K of RAM, others 6K. */
  RAM : ORIGIN = 0x10000000, LENGTH = 8K
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* You may want to use this variable to locate the call stack and static
   variables in different memory regions. Below is shown the default value */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

/* Only vectors and code running at reset are safe to be
   in the first 512 bytes since RAM can be mapped into this
   area for RAM based interrupt vectors. (See user manual
   section 3.5.1 "System memory remap register") */
_stext = ORIGIN(FLASH) + 0x200;

/* Example of putting non-initialized variables into custom RAM locations. */
/* This assumes you have defined a region RAM2 above, and in the Rust
   sources added the attribute `#[link_section = ".ram2bss"]` to the data
   you want to place there. */
/* Note that the section will not be zero-initialized by the runtime! */
/* SECTIONS {
     .ram2bss (NOLOAD) : ALIGN(4) {
       *(.ram2bss);
       . = ALIGN(4);
     } > RAM2
   } INSERT AFTER .bss;
*/
