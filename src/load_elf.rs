// use crate::AnyError;
use std::convert::TryFrom;

use anyhow::{anyhow, Error};

use goblin::elf::{sym, Elf};
use goblin::{Hint, Object};

/// return the tuple: (\
/// 0: the st_value(always means addr) of the sym entry,\
/// 1: the st_size of the sym entry,\
/// 2: elf header e_type\
/// )
pub fn match_sym_entry(bytes: &Vec<u8>, keyword: &String) -> Result<(u64, u64, u16), Error> {
    let elf = parse_elf(&bytes)?;

    let sym_vec = elf.syms.to_vec();
    let satisfied_syms = sym_vec
        .iter()
        .filter(|sym| {
            let name = elf.strtab.get_at(sym.st_name).unwrap_or("");
            name.contains(keyword) && sym.st_type() == sym::STT_OBJECT && sym.st_bind() == sym::STB_GLOBAL
        })
        .collect::<Vec<&sym::Sym>>();

    match satisfied_syms.len() {
        0 => Err(anyhow!("Cannot find {} in sym", keyword)),
        1 => {
            if let Some(sym) = satisfied_syms.get(0) {
                return Ok((sym.st_value, sym.st_size, elf.header.e_type));
            }
            Err(anyhow!("None sym"))
        }
        _len => Err(anyhow!("Find {} counts in sym: {:?}", _len, satisfied_syms)), // TODO: stdin to select?
    }
}

fn parse_elf<'a>(bytes: &'a Vec<u8>) -> Result<Elf<'a>, Error> {
    const MAGIC_LEN: usize = 16;

    let prefix_bytes_ref = bytes
        .get(..MAGIC_LEN)
        .ok_or_else(|| anyhow!("File size is too small: {} bytes", bytes.len()))?;
    let prefix_bytes = <&[u8; MAGIC_LEN]>::try_from(prefix_bytes_ref)?;

    let peek = goblin::peek_bytes(prefix_bytes)?;
    if let Hint::Unknown(magic) = peek {
        return Err(anyhow!("Unknown magic: {:#x}", magic));
    }

    match Object::parse(&bytes).unwrap() {
        Object::Elf(elf) => {
            return Ok(elf);
        }
        _ => return Err(anyhow!("object format error")),
    }
}
