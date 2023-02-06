use {
    color_eyre::Result,
    errctx::PathCtx,
    libarch_sys::root::{
        captive::arch::arm64::{arm64_decode_decode, arm64_disasm_disassemble},
        destruct_decoder, destruct_disassembler, new_decoder, new_disassembler,
    },
    memmap2::Mmap,
    std::{
        env::args,
        ffi::{c_void, CStr},
        fs::File,
    },
};

fn main() -> Result<()> {
    color_eyre::install()?;

    let path = args().skip(1).next().unwrap();

    let file = File::open(&path).map_err(PathCtx::f(&path)).unwrap();
    let mmap = unsafe { Mmap::map(&file) }.unwrap();

    let decoder = unsafe { new_decoder() };
    let disassembler = unsafe { new_disassembler() };

    for chunk in mmap.chunks_exact(4) {
        unsafe { arm64_decode_decode(decoder as *mut c_void, 0, 0, &chunk[0]) };

        let str_ptr = unsafe {
            arm64_disasm_disassemble(disassembler as *mut c_void, 0, decoder as *const u8)
        };

        println!(
            "{:08X}: {}",
            unsafe { std::mem::transmute::<_, &u32>(&chunk[0]) },
            unsafe { CStr::from_ptr(str_ptr) }.to_str().unwrap()
        );
    }

    unsafe { destruct_decoder(decoder) };
    unsafe { destruct_disassembler(disassembler) };

    Ok(())
}