use std::convert::TryFrom;
use std::io;

use anyhow::{anyhow, Error};

use goblin::elf::{header, sym, Elf, SectionHeaders};
use goblin::strtab::Strtab;
use goblin::{Hint, Object};

const MAGIC_LEN: usize = 16;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SymEntry {
    obj_addr: u64,
    obj_size: u64,
    bind_type: u8,
    origin_name: String,
    section: String,
}

impl SymEntry {
    pub fn obj_addr(&self) -> u64 {
        self.obj_addr
    }

    pub fn obj_size(&self) -> u64 {
        self.obj_size
    }
}

pub struct ElfMgr<'a> {
    elf: Elf<'a>,
}

impl<'a> ElfMgr<'a> {
    pub fn prase_from(bytes: &'a [u8]) -> Result<Self, Error> {
        let prefix_bytes_ref = bytes
            .get(..MAGIC_LEN)
            .ok_or_else(|| anyhow!("File size is too small: {} bytes", bytes.len()))?;
        let prefix_bytes = <&[u8; MAGIC_LEN]>::try_from(prefix_bytes_ref)?;

        if let Hint::Unknown(magic) = goblin::peek_bytes(prefix_bytes)? {
            return Err(anyhow!("Unknown magic: {:#x}", magic));
        }

        match Object::parse(bytes).unwrap() {
            Object::Elf(val) => Ok(ElfMgr {
                elf: val,
                // satisfied_entry: SymEntry::default(),
            }),
            _ => Err(anyhow!("Object format not support")),
        }
    }

    pub fn is_exec_elf(&self) -> bool {
        self.elf.header.e_type == header::ET_EXEC
    }

    pub fn is_dyn_elf(&self) -> bool {
        self.elf.header.e_type == header::ET_DYN
    }

    pub fn select_sym_entry(&self, keyword: &String) -> Result<SymEntry, Error> {
        let strtab = &self.elf.strtab;
        let syms = self.elf.syms.to_vec();
        // let dyn_strtab = &self.elf.dynstrtab;
        // let dynsyms = self.elf.dynsyms.to_vec();
        if syms.is_empty() {
            return Err(anyhow!("syms is empty"));
        }

        let map_iter = syms
            .iter()
            .filter_map(|sym| self.filter_symbol(sym, strtab, keyword));

        let emtry_vec: Vec<SymEntry> = map_iter.collect();
        println!("Matched count: {}", emtry_vec.len());
        match emtry_vec.len() {
            0 => Err(anyhow!("cannot find")),
            1 => Ok(emtry_vec.first().unwrap().clone()),
            2.. => {
                for (i, emtry) in emtry_vec.iter().enumerate() {
                    println!(
                        "{}: {:40} | {:7} | {:6} | {}",
                        i,
                        emtry.origin_name,
                        emtry.obj_size,
                        sym::bind_to_str(emtry.bind_type),
                        emtry.section,
                    );
                }
                loop_inquire_index(&emtry_vec)
            }
        }
    }

    fn filter_symbol(&self, sym: &sym::Sym, strtab: &Strtab, keyword: &String) -> Option<SymEntry> {
        // filter: LOCAL&OBJECT or GLOBAL&OBJECT
        if sym.st_type() != sym::STT_OBJECT
            || (sym.st_bind() != sym::STB_LOCAL && sym.st_bind() != sym::STB_GLOBAL)
        {
            return None;
        }

        let sym_symbol = strtab.get_at(sym.st_name).unwrap_or("BAD NAME");
        let (is_demangled, dem_name) = match try_multi_demangle(sym_symbol) {
            Ok(origin) => (true, origin),
            Err(_multi_err) => (false, "".to_string()),
        };
        let shn = shndx_to_str(sym.st_shndx, &self.elf.section_headers, &self.elf.shdr_strtab);

        if is_demangled
            && sym.st_size > 0
            && (shn.starts_with(".bss(") || shn.starts_with(".rodata(") || shn.starts_with(".data"))
            && !(dem_name.contains("(anonymous namespace)")
                // || dem_name.contains("@GLIBC")
                || dem_name.contains("std::")
                || dem_name.starts_with("__gnu_")
                || dem_name.starts_with("__cxxabiv")
                || dem_name.starts_with("guard variable"))
        {
            #[cfg(debug_assertions)]
            eprintln!(
                "{:6} | {:5} | {:12} | {:40} | {}",
                sym::bind_to_str(sym.st_bind()),
                sym.st_size,
                shn,
                dem_name,
                sym_symbol
            );
            if dem_name.contains(keyword) || keyword.is_empty() {
                return Some(SymEntry {
                    obj_addr: sym.st_value,
                    obj_size: sym.st_size,
                    bind_type: sym.st_bind(),
                    origin_name: dem_name.clone(),
                    section: shn.clone(),
                });
            }
        }
        None
    }
}

fn shndx_to_str(idx: usize, shdrs: &SectionHeaders, strtab: &Strtab) -> String {
    if idx == 0 {
        String::from("")
    } else if let Some(shdr) = shdrs.get(idx) {
        if let Some(link_name) = strtab
            .get_at(shdr.sh_name)
            .map(|_str| match try_multi_demangle(_str) {
                Ok(origin) => origin,
                Err(_) => _str.to_string(),
            })
        // TODO: need try_multi_demangle?
        {
            format!("{}({})", link_name, idx)
        } else {
            format!("BAD_IDX={}", shdr.sh_name)
        }
    } else if idx == 0xfff1 {
        // Associated symbol is absolute.
        String::from("ABS")
    } else {
        String::from(&format!("BAD_IDX={}", idx))
    }
}

fn try_multi_demangle(s: &str) -> Result<String, Error> {
    match cpp_demangle::Symbol::new(s) {
        Ok(_symbol) => Ok(_symbol.to_string()),
        Err(cpp_dem_err) => match rustc_demangle::try_demangle(s) {
            Ok(demangled) => Ok(demangled.to_string()),
            Err(rust_dem_err) => Err(anyhow!("`cpp:{:?}, rust:{:?}`", cpp_dem_err, rust_dem_err)),
        },
    }
}

/// the slice's len better greater than 0
fn loop_inquire_index<T>(entry_slice: &[T]) -> Result<T, Error>
where
    T: Clone,
{
    if entry_slice.is_empty() {
        return Err(anyhow!("The slice is empty"));
    }
    println!("Please input index to choose the var(default is 0): ");
    let mut line_input = String::with_capacity(16);
    loop {
        line_input.clear();
        match io::stdin().read_line(&mut line_input) {
            Ok(byte_count) => {
                // eprintln!("{} bytes read", byte_count);
                if let 0 = byte_count {
                    println!("read_line Ok and 0 byte read means EndOfFile");
                    return Err(anyhow!("EOF"));
                }
                let trimmed_input = line_input.trim();
                match trimmed_input.parse::<usize>() {
                    Ok(index) => {
                        if index < entry_slice.len() {
                            return Ok(entry_slice[index].clone());
                        } else {
                            println!(
                                "Input: {} out of range(available is [0, {}]): ",
                                index,
                                entry_slice.len() - 1
                            );
                        }
                    }
                    Err(parse_err) => {
                        if trimmed_input.is_empty() {
                            return Ok(entry_slice[0].clone());
                        }
                        println!("{:?} '{}', try again: ", parse_err, trimmed_input);
                    }
                }
            }
            Err(std_error) => println!("Failed to read line: {:?}", std_error),
        }
    }
}
