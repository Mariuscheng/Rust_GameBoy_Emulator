## Problem
The GameBoy emulator currently renders to the window, but only displays a blank/white screen.

## Root Causes
After analyzing the code, these are the likely issues:

1. **Insufficient VRAM initialization** - The fallback ROM doesn't properly initialize VRAM with visible tile data
2. **Incomplete CPU opcode implementation** - Key opcodes needed for proper VRAM setup may be missing
3. **PPU rendering issues** - The PPU might not be correctly interpreting the VRAM data

## Proposed Solutions

### 1. Add a test pattern ROM implementation

Replace the current fallback ROM with one that explicitly writes visible patterns to VRAM:

```rust
fn create_fallback_rom() -> Vec<u8> {
    let mut fallback = vec![0; 0x8000];
    
    // ROM header area
    fallback[0x100] = 0x00; // Entry point: NOP
    fallback[0x101] = 0x3E; // LD A, value
    fallback[0x102] = 0x91; // value = 0x91 (LCDC value to enable LCD and BG)
    
    // Set LCDC register to enable LCD and background
    fallback[0x103] = 0xE0; // LDH (0xFF00+n), A
    fallback[0x104] = 0x40; // n = 0x40 (0xFF40 is LCDC)
    
    // Set BGP (BG Palette)
    fallback[0x105] = 0x3E; // LD A, value
    fallback[0x106] = 0xE4; // value = 0xE4 (typical GB palette)
    fallback[0x107] = 0xE0; // LDH (0xFF00+n), A
    fallback[0x108] = 0x47; // n = 0x47 (0xFF47 is BGP)
    
    // Write a simple tile pattern to VRAM
    // First set HL to point to tile data area
    fallback[0x109] = 0x21; // LD HL, nn
    fallback[0x10A] = 0x00; // low byte of 0x8000
    fallback[0x10B] = 0x80; // high byte of 0x8000
    
    // Write first tile (checkerboard pattern)
    // Tile data takes 16 bytes (2 bytes per row, 8 rows)
    fallback[0x10C] = 0x3E; // LD A, value
    fallback[0x10D] = 0x55; // value = 0x55 (alternating bits)
    fallback[0x10E] = 0x22; // LD (HL+), A
    fallback[0x10F] = 0x3E; // LD A, value
    fallback[0x110] = 0xAA; // value = 0xAA (opposite alternating bits)
    fallback[0x111] = 0x22; // LD (HL+), A
    
    // Repeat for remaining 7 rows (simplified in this example)
    fallback[0x112] = 0x3E; // LD A, value
    fallback[0x113] = 0xFF; // value = 0xFF (solid row)
    
    for i in 0..14 {
        fallback[0x114 + i*2] = 0x22; // LD (HL+), A
    }
    
    // Write tile ID 1 to background map at position (0,0)
    fallback[0x130] = 0x21; // LD HL, nn
    fallback[0x131] = 0x00; // low byte of 0x9800
    fallback[0x132] = 0x98; // high byte of 0x9800
    fallback[0x133] = 0x3E; // LD A, value
    fallback[0x134] = 0x01; // value = 0x01 (tile ID 1)
    fallback[0x135] = 0x22; // LD (HL+), A
    
    // Write a few more tiles to make pattern visible
    for i in 0..20 {
        fallback[0x136 + i*2] = 0x22; // LD (HL+), A
    }
    
    // Infinite loop
    fallback[0x160] = 0x18; // JR
    fallback[0x161] = 0xFE; // -2 (jump back to self)
    
    // Standard ROM header data
    let title = b"TEST PATTERN";
    for (i, &byte) in title.iter().enumerate() {
        if i < 16 {
            fallback[0x134 + i] = byte;
        }
    }
    
    fallback
}
```

### 2. Complete Missing CPU Opcodes

Add implementations for these important opcodes:
- `0x22` (LD (HL+), A)
- `0x32` (LD (HL-), A)
- `0x2A` (LD A, (HL+))
- `0x21` (LD HL, nn) - already implemented but review
- Instructions for 16-bit operations like LD DE, nn

### 3. Add Debugging Output for PPU

Add detailed logging during rendering to verify:
- When the PPU.step() method is called
- LCDC value and its interpretation
- The first few tile IDs from the background map
- Sample color values being written to the framebuffer

### 4. Add Visual Test Pattern

Add a function to manually write a test pattern to VRAM if nothing appears after X frames:

```rust
fn write_test_pattern_to_vram(&mut self) {
    // Write a simple test pattern to first tile
    let mut vram = self.vram.borrow_mut();
    
    // First tile: solid black (all 1s)
    for i in 0..16 {
        vram[i] = 0xFF;
    }
    
    // Second tile: checkerboard
    for i in (16..32).step_by(2) {
        vram[i] = 0xAA;
        vram[i+1] = 0x55;
    }
    
    // Make first few tiles in BG map point to these test tiles
    for i in 0..10 {
        vram[0x1800 + i] = (i % 2) as u8;
    }
}
```

Call this after X frames if the screen is still blank.

## Additional Diagnostic Steps

1. Add an option to dump the entire VRAM content to a file after X frames
2. Add CPU instruction tracing when running the fallback ROM
3. Verify PPU synchronization is working by tracing values in registers

## Expected Outcome
After implementing these changes, the emulator should at minimum show a test pattern on the screen, confirming that the basic graphics pipeline is working.
