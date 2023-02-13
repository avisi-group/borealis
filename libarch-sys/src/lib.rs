#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use {
    crate::root::{
        captive::arch::arm64::{arm64_decode_decode, arm64_disasm_disassemble},
        new_decoder, new_disassembler,
    },
    std::{
        ffi::{c_void, CStr},
        fmt::Write,
    },
};

pub fn disassemble(data: &[u8]) -> String {
    let mut decode_data = [0u8; 128];
    let mut disasm_data = [0u8; 128];

    let decoder = unsafe { new_decoder(&mut decode_data as *mut u8 as *mut c_void) };
    let disassembler = unsafe { new_disassembler(&mut disasm_data as *mut u8 as *mut c_void) };

    let mut output = String::new();

    for chunk in data.chunks_exact(4) {
        unsafe { arm64_decode_decode(decoder as *mut c_void, 0, 0, &chunk[0]) };

        let str_ptr = unsafe {
            arm64_disasm_disassemble(disassembler as *mut c_void, 0, decoder as *const u8)
        };

        writeln!(
            output,
            "{:08X}: {}",
            unsafe { std::mem::transmute::<_, &u32>(&chunk[0]) },
            unsafe { CStr::from_ptr(str_ptr) }.to_str().unwrap()
        )
        .unwrap();
    }

    output
}

#[cfg(test)]
mod tests {
    #[test]
    fn decode() {
        // Aarch64 assembly for
        //
        // ```
        // #include <unistd.h>
        //
        // int main()
        // {
        //     const char msg[] = "Hello, ARM!\n";
        //     __uint32_t a = 123 + 37;
        //     __uint32_t b = a * 7;
        //     __uint32_t c = a / b;
        //     write(0, msg, sizeof(msg));
        //     _exit(0);
        // }
        // ```
        let data = [
            0xFD, 0x7B, 0xBC, 0xA9, 0xFD, 0x03, 0x00, 0x91, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00,
            0x40, 0xF9, 0x01, 0x00, 0x40, 0xF9, 0xE1, 0x1F, 0x00, 0xF9, 0x01, 0x00, 0x80, 0xD2,
            0x00, 0x00, 0x00, 0x90, 0x01, 0x00, 0x00, 0x91, 0xE0, 0xA3, 0x00, 0x91, 0x22, 0x00,
            0x40, 0xF9, 0x02, 0x00, 0x00, 0xF9, 0x21, 0x50, 0x40, 0xF8, 0x01, 0x50, 0x00, 0xF8,
            0x00, 0x14, 0x80, 0x52, 0xE0, 0x1F, 0x00, 0xB9, 0xE1, 0x1F, 0x40, 0xB9, 0xE0, 0x03,
            0x01, 0x2A, 0x00, 0x70, 0x1D, 0x53, 0x00, 0x00, 0x01, 0x4B, 0xE0, 0x23, 0x00, 0xB9,
            0xE1, 0x1F, 0x40, 0xB9, 0xE0, 0x23, 0x40, 0xB9, 0x20, 0x08, 0xC0, 0x1A, 0xE0, 0x27,
            0x00, 0xB9, 0xE0, 0xA3, 0x00, 0x91, 0xA2, 0x01, 0x80, 0xD2, 0xE1, 0x03, 0x00, 0xAA,
            0x00, 0x00, 0x80, 0x52, 0x00, 0x00, 0x00, 0x94, 0x00, 0x00, 0x80, 0x52, 0x00, 0x00,
            0x00, 0x94,
        ];

        insta::assert_snapshot!(crate::disassemble(&data));
    }
}
