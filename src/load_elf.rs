use crate::AnyError;

use anyhow::anyhow;

use goblin::elf::*;
use goblin::{Hint, Object};
use goblin::strtab::Strtab;
use std::convert::TryFrom;


fn fmt_symbol(sym: &sym::Sym, strtab: &Strtab) {
    let name = strtab.get_at(sym.st_name).unwrap_or("BAD NAME");
    println!("{:?} {:?} {:?}", sym.st_bind().to_string(), sym.st_type().to_string(), name);
}

pub fn run_parse(in_file: &str) -> AnyError {
    let bytes = std::fs::read(in_file)
        .map_err(|err| anyhow::anyhow!("Problem reading file {:?}: {}", in_file, err))?;

    let prefix_bytes_ref = bytes.get(..16).ok_or_else(|| {
        anyhow!(
            "File size is too small {:?}: {} bytes",
            in_file,
            bytes.len()
        )
    })?;
    let prefix_bytes = <&[u8; 16]>::try_from(prefix_bytes_ref)?;

    let peek = goblin::peek_bytes(prefix_bytes)?;
    if let Hint::Unknown(magic) = peek {
        return Err(anyhow::anyhow!("Unknown magic: {:#x}", magic));
    }

    let object = Object::parse(&bytes)?;

    match object {
        Object::Elf(elf) => {
            // Elf::new(elf, bytes, opt).print(),
            let syms = elf.syms.to_vec();
            let strtab = elf.strtab;
            syms.iter().for_each(|sym| fmt_symbol(sym, &strtab));
        }
        _ => return Err(anyhow!("object format error")),
    }
    Ok(())
}
